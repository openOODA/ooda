/* M17: process-local AllocCap — explicit alloc helpers only.
 * Not OS rlimit / heap isolation / ASAN. Ambient list_new remains free. */
#include "chs_rt.h"
#include <unistd.h>
#if defined(__linux__)
#include <sys/random.h>
#endif

static long long g_tok_alloc;
static int g_alloc_ready;

/* Classic forgeable magic OOAL — must never be the live token. */
#define OO_CLASSIC_ALLOC 0x4F4F414CLL

static void oo_alloc_init(void) {
  unsigned char b[8];
  size_t i;
  unsigned long long acc;
  if (g_alloc_ready) return;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)(uintptr_t)&g_alloc_ready;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  /* 0x7… band (fs=1 sys=2 env=3 net=4 time=5 rand=6 alloc=7) */
  g_tok_alloc = 0x700000000LL
      | (long long)((b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3]);
  if (g_tok_alloc == OO_CLASSIC_ALLOC) g_tok_alloc ^= 0x11111111LL;
  g_alloc_ready = 1;
}

long long oo_cap_grant_alloc(void) {
  oo_alloc_init();
  return g_tok_alloc;
}

void oo_cap_require_alloc(long long got, const char *op) {
  oo_alloc_init();
  if (got != g_tok_alloc) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n",
            op ? op : "alloc");
    exit(1);
  }
}

/* Smoke-friendly: re-check cap then return n as opaque size token (not real mmap). */
long long oo_alloc_bytes(long long cap, long long n) {
  oo_cap_require_alloc(cap, "alloc_bytes");
  if (n < 0) n = 0;
  return n;
}

/* Smoke-friendly: re-check cap; free is a no-op by handle. */
void oo_free_bytes(long long cap, long long p) {
  oo_cap_require_alloc(cap, "free_bytes");
  (void)p;
}
