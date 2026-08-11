/* M156: process-local UnsafeFFICap + stub oo_dlopen (not OS dlopen / full C TCB). */
#include "chs_rt.h"
#include <unistd.h>
#if defined(__linux__) || defined(__APPLE__)
#include <sys/random.h>
#endif

static long long g_tok_ffi;
static int g_ffi_ready;

static void oo_ffi_init(void) {
  unsigned char b[8];
  size_t i;
  unsigned long long acc;
  if (g_ffi_ready) return;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)(uintptr_t)&g_tok_ffi;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  g_tok_ffi = 0x500000000LL | (long long)((b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3]);
  if (g_tok_ffi == 0x4F4F4649LL) g_tok_ffi ^= 0x11111111LL;
  g_ffi_ready = 1;
}

long long oo_cap_grant_ffi(void) {
  oo_ffi_init();
  return g_tok_ffi;
}

void oo_cap_require_ffi(long long got, const char *op) {
  oo_ffi_init();
  if (got != g_tok_ffi) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n",
            op ? op : "ffi");
    exit(1);
  }
}

/* Seal-checked stub — process-local only; not real OS dlopen. */
OoResS oo_dlopen(long long cap, OoStr path) {
  OoResS r;
  oo_cap_require_ffi(cap, "dlopen");
  r.ok = 0;
  r.val = oo_str_lit("ffi residual: process-local seal only (no OS dlopen)");
  (void)path;
  return r;
}
