/* Path-A metamorphic floor: process-local epoch for future immune layouts.
 * Not runtime assembly mutation. Full DESIGN metamorphic product is residual. */
#include "chs_rt.h"
#include <unistd.h>
#if defined(__linux__) || defined(__APPLE__)
#include <sys/random.h>
#endif

static long long g_meta_epoch;
static int g_meta_ready;

static void oo_meta_init(void) {
  unsigned char b[8];
  size_t i;
  unsigned long long acc;
  if (g_meta_ready) return;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)(uintptr_t)&g_meta_epoch;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  g_meta_epoch = (long long)((((unsigned long long)b[0]) << 56)
      | (((unsigned long long)b[1]) << 48)
      | (((unsigned long long)b[2]) << 40)
      | (((unsigned long long)b[3]) << 32)
      | (((unsigned long long)b[4]) << 24)
      | (((unsigned long long)b[5]) << 16)
      | (((unsigned long long)b[6]) << 8)
      | ((unsigned long long)b[7]));
  if (g_meta_epoch == 0) g_meta_epoch = 1;
  g_meta_ready = 1;
}

/* Process-local random epoch fixed at first call. For layout/immune hooks.
 * Residual: does not re-morph code after load. */
long long oo_meta_epoch(void) {
  oo_meta_init();
  return g_meta_epoch;
}
