/* M161 library path A: net/process/thread/gpu sealed stubs (+ ThreadCap/GpuCap). */
#include "chs_rt.h"
#include <unistd.h>
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

static OoResS residual_net(long long cap, const char *op) {
  OoResS r;
  oo_cap_require_net(cap, op);
  r.ok = 0;
  r.val = oo_str_lit("net residual: path A seal only (no full TCP/UDP/TLS product)");
  return r;
}
OoResS oo_tcp_bind(long long cap, long long port) {
  (void)port; return residual_net(cap, "tcp_bind");
}
OoResS oo_tcp_connect(long long cap, OoStr host, long long port) {
  (void)host; (void)port; return residual_net(cap, "tcp_connect");
}
OoResS oo_bind_udp(long long cap, long long port) {
  (void)port; return residual_net(cap, "bind_udp");
}
OoResS oo_tls_connect(long long cap, OoStr host, long long port) {
  (void)host; (void)port; return residual_net(cap, "tls_connect");
}

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

OoResS oo_mutex_lock(long long cap, long long mid) {
  OoResS r;
  oo_cap_require_thread(cap, "mutex_lock");
  r.ok = 0;
  r.val = oo_str_lit("mutex residual: path A seal only");
  (void)mid;
  return r;
}
OoResS oo_mutex_unlock(long long cap, long long mid) {
  OoResS r;
  oo_cap_require_thread(cap, "mutex_unlock");
  r.ok = 0;
  r.val = oo_str_lit("mutex residual: path A seal only");
  (void)mid;
  return r;
}
OoResS oo_thread_spawn(long long cap, OoStr name) {
  OoResS r;
  oo_cap_require_thread(cap, "thread_spawn");
  r.ok = 0;
  r.val = oo_str_lit("thread_spawn residual: path A seal only");
  (void)name;
  return r;
}
OoResS oo_gpu_launch(long long cap, OoStr shader) {
  OoResS r;
  oo_cap_require_gpu(cap, "gpu_launch");
  r.ok = 0;
  r.val = oo_str_lit("gpu residual: path A seal only");
  (void)shader;
  return r;
}
