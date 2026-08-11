/* M161/M162: ThreadCap/GpuCap + process residual; pthread mutex path A.
 * Joinable threads: see chs_rt_thread.c (M163). */
#include "chs_rt.h"
#include <unistd.h>
#include <pthread.h>
#if defined(__linux__) || defined(__APPLE__)
#include <sys/random.h>
#endif

static long long g_tok_thread, g_tok_gpu;
static int g_tg_ready;

static void oo_tg_init(void) {
  unsigned char b[16];
  size_t i;
  unsigned long long acc;
  if (g_tg_ready) return;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)(uintptr_t)&g_tok_thread;
    acc ^= (unsigned long long)getpid() << 12;
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  g_tok_thread = 0x600000000LL | (long long)((b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3]);
  g_tok_gpu = 0x700000000LL | (long long)((b[4] << 24) | (b[5] << 16) | (b[6] << 8) | b[7]);
  if (g_tok_thread == 0x4F4F5448LL) g_tok_thread ^= 0x11111111LL;
  if (g_tok_gpu == 0x4F4F4750LL) g_tok_gpu ^= 0x11111111LL;
  g_tg_ready = 1;
}

long long oo_cap_grant_thread(void) { oo_tg_init(); return g_tok_thread; }
long long oo_cap_grant_gpu(void) { oo_tg_init(); return g_tok_gpu; }
void oo_cap_require_thread(long long got, const char *op) {
  oo_tg_init();
  if (got != g_tok_thread) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "thread");
    exit(1);
  }
}
void oo_cap_require_gpu(long long got, const char *op) {
  oo_tg_init();
  if (got != g_tok_gpu) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "gpu");
    exit(1);
  }
}

/* Process residual seals (real process still via sys_exec) */
OoResS oo_sys_spawn(long long cap, OoStr cmd) {
  OoResS r;
  oo_cap_require_sys(cap, "sys_spawn");
  r.ok = 0;
  r.val = oo_str_lit("sys_spawn residual: use sys_exec for blocking spawn+wait");
  (void)cmd;
  return r;
}
OoResS oo_sys_wait(long long cap, long long pid) {
  OoResS r;
  oo_cap_require_sys(cap, "sys_wait");
  r.ok = 0;
  r.val = oo_str_lit("sys_wait residual: path A seal only");
  (void)pid;
  return r;
}
OoResS oo_sys_kill(long long cap, long long pid, long long sig) {
  OoResS r;
  oo_cap_require_sys(cap, "sys_kill");
  r.ok = 0;
  r.val = oo_str_lit("sys_kill residual: path A seal only");
  (void)pid; (void)sig;
  return r;
}

/* M166 path A: OS syscall free names — SysCap require then residual Err.
 * Honesty: not full async I/O / epoll loop / inotify watches / prctl product. */
OoResS oo_sys_epoll_create(long long cap, long long flags) {
  OoResS r;
  oo_cap_require_sys(cap, "sys_epoll_create");
  r.ok = 0;
  r.val = oo_str_lit("sys_epoll_create residual: not full async I/O");
  (void)flags;
  return r;
}
OoResS oo_sys_inotify_init(long long cap) {
  OoResS r;
  oo_cap_require_sys(cap, "sys_inotify_init");
  r.ok = 0;
  r.val = oo_str_lit("sys_inotify_init residual: path A seal only");
  return r;
}
OoResS oo_sys_prctl(long long cap, long long option) {
  OoResS r;
  oo_cap_require_sys(cap, "sys_prctl");
  r.ok = 0;
  r.val = oo_str_lit("sys_prctl residual: path A seal only");
  (void)option;
  return r;
}

#define OO_MUTEX_SLOTS 64
static pthread_mutex_t g_mutexes[OO_MUTEX_SLOTS];
static int g_mutex_inited[OO_MUTEX_SLOTS];
static pthread_mutex_t g_mutex_boot = PTHREAD_MUTEX_INITIALIZER;

static pthread_mutex_t *mutex_for(long long mid) {
  unsigned idx = (unsigned)(mid < 0 ? -mid : mid) % OO_MUTEX_SLOTS;
  pthread_mutex_lock(&g_mutex_boot);
  if (!g_mutex_inited[idx]) {
    pthread_mutex_init(&g_mutexes[idx], NULL);
    g_mutex_inited[idx] = 1;
  }
  pthread_mutex_unlock(&g_mutex_boot);
  return &g_mutexes[idx];
}

OoResS oo_mutex_lock(long long cap, long long mid) {
  OoResS r;
  oo_cap_require_thread(cap, "mutex_lock");
  if (pthread_mutex_lock(mutex_for(mid)) != 0) {
    r.ok = 0;
    r.val = oo_str_lit("mutex_lock failed");
    return r;
  }
  r.ok = 1;
  r.val = oo_str_lit("locked");
  return r;
}
OoResS oo_mutex_unlock(long long cap, long long mid) {
  OoResS r;
  oo_cap_require_thread(cap, "mutex_unlock");
  if (pthread_mutex_unlock(mutex_for(mid)) != 0) {
    r.ok = 0;
    r.val = oo_str_lit("mutex_unlock failed");
    return r;
  }
  r.ok = 1;
  r.val = oo_str_lit("unlocked");
  return r;
}

/* M165 Path A: noop / cpu: fallthrough honesty; no device shaders (no CUDA). */
OoResS oo_gpu_launch(long long cap, OoStr shader) {
  OoResS r;
  const char *p;
  long long len;
  char buf[96];
  oo_cap_require_gpu(cap, "gpu_launch");
  p = shader.data ? shader.data : "";
  len = shader.len;
  if (len < 0) len = 0;
  /* empty or "noop" → Ok("gpu-noop") — no device, honesty product path */
  if (len == 0 || (len == 4 && strncmp(p, "noop", 4) == 0)) {
    r.ok = 1;
    r.val = oo_str_lit("gpu-noop");
    return r;
  }
  /* cpu:… → trivial CPU interpret; honesty "cpu fallthrough" (not GPU) */
  if (len >= 4 && strncmp(p, "cpu:", 4) == 0) {
    const char *rest = p + 4;
    long long rest_len = len - 4;
    if (rest_len >= 4 && strncmp(rest, "add:", 4) == 0) {
      const char *nums = rest + 4;
      char *end1 = NULL;
      long long a, b;
      a = strtoll(nums, &end1, 10);
      if (end1 && *end1 == ':') {
        b = strtoll(end1 + 1, NULL, 10);
        snprintf(buf, sizeof buf, "cpu fallthrough:%lld", a + b);
        r.ok = 1;
        r.val = oo_str_lit(buf);
        return r;
      }
    }
    r.ok = 1;
    r.val = oo_str_lit("cpu fallthrough");
    return r;
  }
  /* PTX/SPIR-V/device shaders still fail-closed residual */
  r.ok = 0;
  r.val = oo_str_lit("gpu residual: no device shaders");
  return r;
}
