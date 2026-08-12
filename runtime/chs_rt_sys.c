#include "chs_rt.h"
#include <errno.h>
#include <netdb.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <pthread.h>
#if defined(__linux__)
#include <sys/random.h>
#endif

/* Process-local capability tokens (R1). Fixed magic ints no longer grant.
 * Time/Rand tokens live in chs_rt_time_rand.c (M12). */
static pthread_once_t g_caps_once = PTHREAD_ONCE_INIT;
static long long g_tok_fs, g_tok_sys, g_tok_env, g_tok_net;

static void caps_once_init(void) {
  unsigned char b[32];
  size_t i;
  unsigned long long acc;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    /* Fallback: ASLR + pid + clock — still not a fixed published magic. */
    acc = (unsigned long long)(uintptr_t)&g_tok_fs;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  g_tok_fs = 0x1000000000000000LL | (long long)((((unsigned long long)b[0]) << 56) | (((unsigned long long)b[1]) << 48) | (((unsigned long long)b[2]) << 40) | (((unsigned long long)b[3]) << 32) | (((unsigned long long)b[4]) << 24) | (((unsigned long long)b[5]) << 16) | (((unsigned long long)b[6]) << 8) | ((unsigned long long)b[7]));
  g_tok_sys = 0x2000000000000000LL | (long long)((((unsigned long long)b[8]) << 56) | (((unsigned long long)b[9]) << 48) | (((unsigned long long)b[10]) << 40) | (((unsigned long long)b[11]) << 32) | (((unsigned long long)b[12]) << 24) | (((unsigned long long)b[13]) << 16) | (((unsigned long long)b[14]) << 8) | ((unsigned long long)b[15]));
  g_tok_env = 0x3000000000000000LL | (long long)((((unsigned long long)b[16]) << 56) | (((unsigned long long)b[17]) << 48) | (((unsigned long long)b[18]) << 40) | (((unsigned long long)b[19]) << 32) | (((unsigned long long)b[20]) << 24) | (((unsigned long long)b[21]) << 16) | (((unsigned long long)b[22]) << 8) | ((unsigned long long)b[23]));
  g_tok_net = 0x4000000000000000LL | (long long)((((unsigned long long)b[24]) << 56) | (((unsigned long long)b[25]) << 48) | (((unsigned long long)b[26]) << 40) | (((unsigned long long)b[27]) << 32) | (((unsigned long long)b[28]) << 24) | (((unsigned long long)b[29]) << 16) | (((unsigned long long)b[30]) << 8) | ((unsigned long long)b[31]));
  /* Never equal classic forgeable magics */
  if (g_tok_fs == 0x4F4F4653LL) g_tok_fs ^= 0x11111111LL;
  if (g_tok_sys == 0x4F4F5359LL) g_tok_sys ^= 0x11111111LL;
  if (g_tok_env == 0x4F4F454ELL) g_tok_env ^= 0x11111111LL;
  if (g_tok_net == 0x4F4F4E54LL) g_tok_net ^= 0x11111111LL;
}

static void oo_caps_init(void) {
  pthread_once(&g_caps_once, caps_once_init);
}

long long oo_cap_grant_fs(void) { oo_caps_init(); return g_tok_fs; }
long long oo_cap_grant_sys(void) { oo_caps_init(); return g_tok_sys; }
long long oo_cap_grant_env(void) { oo_caps_init(); return g_tok_env; }
long long oo_cap_grant_net(void) { oo_caps_init(); return g_tok_net; }

void oo_cap_require(long long got, long long want, const char *op) {
  oo_caps_init();
  if (got != want) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "?");
    exit(1);
  }
}

/* Kind-based require used by fs/env (want token from grant table). */
void oo_cap_require_fs(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_fs) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "fs");
    exit(1);
  }
}
void oo_cap_require_sys(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_sys) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "sys");
    exit(1);
  }
}
void oo_cap_require_env(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_env) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "env");
    exit(1);
  }
}

/* ZT path A: process-policy getenv — fail-closed for non OODA_/OO_ keys.
 * Product env_get still requires EnvCap via oo_env_get. */
const char *oo_process_policy_getenv(const char *key) {
  if (!key || !key[0]) return NULL;
  if (strncmp(key, "OODA_", 5) != 0 && strncmp(key, "OO_", 3) != 0) {
    return NULL;
  }
  return getenv(key);
}
void oo_cap_require_net(long long got, const char *op) {
  oo_caps_init();
  if (got != g_tok_net) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "net");
    exit(1);
  }
}

/* R2/R3: fork + execvp with full argv (no system(3) shell). */
OoResS oo_sys_exec(long long cap, int argc, OoStr *argv) {
  OoResS r;
  char **av;
  int i, st;
  pid_t pid;
  oo_cap_require_sys(cap, "sys_exec");
  r.ok = 0;
  r.val = oo_str_lit("sys_exec failed");
  if (argc < 1 || !argv || !argv[0].data) return r;
  av = (char **)calloc((size_t)argc + 1, sizeof(char *));
  if (!av) return r;
  for (i = 0; i < argc; i++) {
    av[i] = argv[i].data ? argv[i].data : (char *)"";
  }
  av[argc] = NULL;
  pid = fork();
  if (pid < 0) {
    free(av);
    return r;
  }
  if (pid == 0) {
    /* DE1.8: filter env before execvp so child does not inherit full parent env.
     * Walk original environ, keep only OODA_/OO_ keys per oo_process_policy_getenv,
     * then set a minimal PATH so the child can locate the executable. */
    extern char **environ;
    char **src, **saved = environ;
    char **newenv = NULL;
    size_t n = 0, cap = 0;
#if defined(__GLIBC__) || defined(__APPLE__)
    clearenv();
#else
    environ = NULL;
#endif
    if (saved) {
      for (src = saved; *src; src++) {
        const char *eq = strchr(*src, '=');
        size_t klen;
        char kbuf[256];
        if (!eq) continue;
        klen = (size_t)(eq - *src);
        if (klen == 0 || klen >= sizeof kbuf) continue;
        memcpy(kbuf, *src, klen);
        kbuf[klen] = 0;
        if (oo_process_policy_getenv(kbuf) == NULL) continue;
        if (n + 1 >= cap) {
          cap = cap ? cap * 2 : 16;
          newenv = (char **)realloc(newenv, cap * sizeof(char *));
          if (!newenv) _exit(127);
        }
        newenv[n++] = *src;
      }
    }
    if (newenv) {
      newenv[n] = NULL;
      environ = newenv;
    }
    setenv("PATH", "/usr/bin:/bin", 1);
    execvp(av[0], av);
    _exit(127);
  }
  free(av);
  if (waitpid(pid, &st, 0) < 0) return r;
  if (WIFEXITED(st) && WEXITSTATUS(st) == 0) {
    r.ok = 1;
    r.val = oo_str_lit("");
  }
  return r;
}

/* Compat: single command string via exec of sh -c (still argv form, not system). */
OoResS oo_sys_exec1(long long cap, OoStr cmd) {
  OoStr av[3];
  av[0] = oo_str_lit("sh");
  av[1] = oo_str_lit("-c");
  av[2] = cmd;
  return oo_sys_exec(cap, 3, av);
}

/* R9: real HTTP/1.0 GET via POSIX sockets (http only; https fail-closed). */
OoResS oo_fetch(long long cap, OoStr url) {
  OoResS r;
  const char *u;
  char host[256], path[1024], portstr[8];
  int port = 80, fd = -1, n;
  size_t ulen, i, j;
  struct addrinfo hints, *res = NULL, *rp;
  char req[1400], *body = NULL, *acc = NULL;
  size_t acc_len = 0, acc_cap = 0;
  ssize_t nr;
  oo_cap_require_net(cap, "fetch");
  r.ok = 0;
  r.val = oo_str_lit("fetch failed");
  u = url.data ? url.data : "";
  ulen = url.data ? (size_t)url.len : 0;
  if (ulen >= 8 && strncmp(u, "https://", 8) == 0) {
    r.val = oo_str_lit("https residual: use http:// or external TLS");
    return r;
  }
  if (ulen < 7 || strncmp(u, "http://", 7) != 0) {
    r.val = oo_str_lit("fetch: only http:// URLs supported");
    return r;
  }
  u += 7;
  ulen -= 7;
  i = 0;
  while (i < ulen && u[i] != '/' && u[i] != ':' && i < sizeof(host) - 1) {
    host[i] = u[i];
    i++;
  }
  host[i] = 0;
  if (i == 0) return r;
  if (i < ulen && u[i] == ':') {
    i++;
    j = 0;
    while (i < ulen && u[i] != '/' && j < sizeof(portstr) - 1) {
      portstr[j++] = u[i++];
    }
    portstr[j] = 0;
    port = atoi(portstr);
    if (port <= 0) port = 80;
  }
  if (i < ulen && u[i] == '/') {
    j = 0;
    while (i < ulen && j < sizeof(path) - 1) path[j++] = u[i++];
    path[j] = 0;
  } else {
    path[0] = '/';
    path[1] = 0;
  }
  memset(&hints, 0, sizeof hints);
  hints.ai_socktype = SOCK_STREAM;
  snprintf(portstr, sizeof portstr, "%d", port);
  if (getaddrinfo(host, portstr, &hints, &res) != 0) {
    r.val = oo_str_lit("fetch: DNS failed");
    return r;
  }
  for (rp = res; rp; rp = rp->ai_next) {
    fd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol);
    if (fd < 0) continue;
    if (connect(fd, rp->ai_addr, rp->ai_addrlen) == 0) break;
    close(fd);
    fd = -1;
  }
  freeaddrinfo(res);
  if (fd < 0) {
    r.val = oo_str_lit("connection refused");
    return r;
  }
  n = snprintf(req, sizeof req,
               "GET %s HTTP/1.0\r\nHost: %s\r\nConnection: close\r\n\r\n", path, host);
  if (n <= 0 || (size_t)n >= sizeof req || write(fd, req, (size_t)n) != n) {
    close(fd);
    return r;
  }
  acc_cap = 4096;
  acc = (char *)malloc(acc_cap);
  if (!acc) {
    close(fd);
    return r;
  }
  while ((nr = read(fd, req, sizeof req)) > 0) {
    if (acc_len + (size_t)nr + 1 > acc_cap) {
      acc_cap = (acc_len + (size_t)nr + 1) * 2;
      acc = (char *)realloc(acc, acc_cap);
      if (!acc) {
        close(fd);
        return r;
      }
    }
    memcpy(acc + acc_len, req, (size_t)nr);
    acc_len += (size_t)nr;
  }
  close(fd);
  acc[acc_len] = 0;
  body = strstr(acc, "\r\n\r\n");
  if (!body) {
    free(acc);
    r.val = oo_str_lit("fetch: bad response");
    return r;
  }
  body += 4;
  {
    size_t blen = acc_len - (size_t)(body - acc);
    char *out = oo_str_alloc_payload(blen);
    memcpy(out, body, blen);
    free(acc);
    r.ok = 1;
    r.val.data = out;
    r.val.len = (long long)blen;
  }
  return r;
}
