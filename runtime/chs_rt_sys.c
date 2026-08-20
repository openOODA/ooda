#include "chs_rt.h"
#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <limits.h>

/* ZT path A: process-policy getenv — fail-closed for non OODA_/OO_ keys.
 * Product env_get still requires EnvCap via oo_env_get. */
const char *oo_process_policy_getenv(const char *key) {
  if (!key || !key[0]) return NULL;
  if (strncmp(key, "OODA_", 5) != 0 && strncmp(key, "OO_", 3) != 0) {
    return NULL;
  }
  return getenv(key);
}

/* Child of sys_exec / sys_spawn: keep OODA_/OO_ keys only, then PATH=/usr/bin:/bin. */
void oo_child_filter_env(void) {
  extern char **environ;
  char **src;
  char **newenv = NULL;
  size_t n = 0, env_cap = 0;
  if (environ) {
    for (src = environ; *src; src++) {
      const char *eq = strchr(*src, '=');
      size_t klen;
      if (!eq) continue;
      klen = (size_t)(eq - *src);
      if (klen == 0) continue;
      if (!((klen >= 5 && strncmp(*src, "OODA_", 5) == 0) ||
            (klen >= 3 && strncmp(*src, "OO_", 3) == 0)))
        continue;
      if (n + 1 >= env_cap) {
        env_cap = env_cap ? env_cap * 2 : 16;
        newenv = (char **)realloc(newenv, env_cap * sizeof(char *));
        if (!newenv) _exit(127);
      }
      newenv[n++] = *src;
    }
  }
#if defined(__GLIBC__) || defined(__APPLE__)
  clearenv();
#else
  if (environ) {
    environ[0] = NULL;
  }
#endif
  if (newenv) {
    newenv[n] = NULL;
    environ = newenv;
  }
  setenv("PATH", "/usr/bin:/bin", 1);
}

/* Shared by write_file (chs_rt_fs.c) in this TU. Untrusted blob is OODA_UNTRUSTED. */
int oo_blob_has(const char *data, size_t n, const char *u, size_t ul) {
  size_t i;
  if (!data || !u || ul == 0 || ul > n) return 0;
  for (i = 0; i + ul <= n; i++) {
    if (memcmp(data + i, u, ul) == 0) return 1;
  }
  return 0;
}

int oo_untrusted_hit(const char *data, size_t n) {
  const char *u = oo_process_policy_getenv("OODA_UNTRUSTED");
  if (!u || !u[0] || !data) return 0;
  return oo_blob_has(data, n, u, strlen(u));
}

int oo_policy_write_on(void) {
  const char *v = oo_process_policy_getenv("OODA_POLICY_WRITE");
  return v && v[0] == '1' && v[1] == '\0';
}

int oo_is_policy_path(const char *p) {
  const char *b;
  size_t n;
  if (!p || !p[0]) return 0;
  if (strstr(p, "/.config/ooda/")) return 1;
  b = strrchr(p, '/');
  b = b ? b + 1 : p;
  if (strcmp(b, "SOUL.md") == 0 || strcmp(b, "soul.md") == 0) return 1;
  if (strcmp(b, ".bashrc") == 0 || strcmp(b, "ooda.lock") == 0) return 1;
  n = strlen(b);
  if (n >= 10 && strcmp(b + (n - 10), ".agent.pin") == 0) return 1;
  return 0;
}

/* R2/R3: fork + execvp with full argv (no system(3) shell). */
OoResS oo_sys_exec(long long cap, int argc, OoStr *argv) {
  OoResS r;
  char **av;
  int i, st;
  long long k;
  pid_t pid;
  oo_cap_require_process(cap, "sys_exec");
  r.ok = 0;
  r.val = oo_str_lit("sys_exec failed");
  if (argc < 1 || !argv || !argv[0].data) return r;
  av = (char **)calloc((size_t)argc + 1, sizeof(char *));
  if (!av) return r;
  for (i = 0; i < argc; i++) {
    /* Fail-closed like to_cpath: empty / len<=0 / NUL inside .len → no exec. */
    if (!argv[i].data || argv[i].len <= 0) {
      free(av);
      return r;
    }
    for (k = 0; k < argv[i].len; k++) {
      if (argv[i].data[k] == '\0') {
        free(av);
        return r;
      }
    }
    if (oo_untrusted_hit(argv[i].data, (size_t)argv[i].len)) {
      free(av);
      r.val = oo_str_lit("process_exec denied: untrusted");
      return r;
    }
    av[i] = argv[i].data;
  }
  av[argc] = NULL;
  pid = fork();
  if (pid < 0) {
    free(av);
    return r;
  }
  if (pid == 0) {
    oo_child_filter_env();
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

OoSList oo_sys_args(long long cap) {
  oo_cap_require_process(cap, "sys_args");
  OoSList l = oo_slist_new();
  FILE *f = fopen("/proc/self/cmdline", "rb");
  if (!f) return l;
  char buf[4096];
  size_t n = fread(buf, 1, sizeof(buf) - 1, f);
  fclose(f);
  if (n == 0) return l;
  size_t start = 0;
  size_t i;
  int first = 1;
  for (i = 0; i < n; i++) {
    if (buf[i] == '\0') {
      if (!first) {
        OoStr arg = oo_str_lit(buf + start);
        OoSList next = oo_slist_push(l, arg);
        oo_slist_release(l);
        l = next;
        oo_str_release(arg);
      }
      first = 0;
      start = i + 1;
    }
  }
  return l;
}

#include <sys/stat.h>

/* Read all stdin (stdio LSP / one-shot). Pipes have no seek. */
OoStr oo_read_stdin(void) {
  char *buf;
  size_t cap = 4096;
  size_t n = 0;
  buf = (char *)malloc(cap);
  if (!buf) return oo_str_lit("");
  for (;;) {
    size_t got;
    if (n + 1024 >= cap) {
      char *nb;
      cap *= 2;
      if (cap > (1u << 20)) {
        free(buf);
        return oo_str_lit("");
      }
      nb = (char *)realloc(buf, cap);
      if (!nb) {
        free(buf);
        return oo_str_lit("");
      }
      buf = nb;
    }
    got = fread(buf + n, 1, 1024, stdin);
    n += got;
    if (got < 1024) break;
  }
  {
    OoStr r;
    r.data = buf;
    r.len = (long long)n;
    return r;
  }
}

/* Fast cache key: size:mtime. Avoids hashing whole compiler sources. */
OoStr oo_file_stamp(OoStr path) {
  char cpath[PATH_MAX];
  struct stat st;
  char buf[64];
  if (!path.data || path.len <= 0 || path.len >= PATH_MAX) return oo_str_lit("0:0");
  memcpy(cpath, path.data, (size_t)path.len);
  cpath[path.len] = 0;
  if (stat(cpath, &st) != 0) return oo_str_lit("0:0");
  snprintf(buf, sizeof buf, "%lld:%lld", (long long)st.st_size, (long long)st.st_mtime);
  return oo_str_lit(buf);
}

void oo_process_exit(long long c) {
  exit((int)c);
}
