#include "chs_rt.h"
#include <errno.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

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
