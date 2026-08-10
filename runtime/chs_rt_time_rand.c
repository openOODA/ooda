/* M12: process-local TimeCap / RandCap — wall clock + entropy (not crypto object-caps) */
#include "chs_rt.h"
#include <time.h>
#include <unistd.h>
#if defined(__linux__)
#include <sys/random.h>
#endif

static long long g_tok_time, g_tok_rand;
static int g_tr_ready;
static unsigned long long g_prng = 1;

static void oo_tr_init(void) {
  unsigned char b[16];
  size_t i;
  unsigned long long acc;
  if (g_tr_ready) return;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)(uintptr_t)&g_tr_ready;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  g_tok_time = 0x500000000LL | (long long)((b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3]);
  g_tok_rand = 0x600000000LL | (long long)((b[4] << 24) | (b[5] << 16) | (b[6] << 8) | b[7]);
  if (g_tok_time == 0x4F4F544DLL) g_tok_time ^= 0x11111111LL;
  if (g_tok_rand == 0x4F4F524ELL) g_tok_rand ^= 0x11111111LL;
  g_prng = 1ULL | ((unsigned long long)b[8] << 8) | b[9];
  g_tr_ready = 1;
}

long long oo_cap_grant_time(void) { oo_tr_init(); return g_tok_time; }
long long oo_cap_grant_rand(void) { oo_tr_init(); return g_tok_rand; }

void oo_cap_require_time(long long got, const char *op) {
  oo_tr_init();
  if (got != g_tok_time) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "time");
    exit(1);
  }
}
void oo_cap_require_rand(long long got, const char *op) {
  oo_tr_init();
  if (got != g_tok_rand) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "rand");
    exit(1);
  }
}

long long oo_now_ms(long long cap) {
  struct timespec ts;
  oo_cap_require_time(cap, "now_ms");
  if (clock_gettime(CLOCK_REALTIME, &ts) != 0) return 0;
  return (long long)ts.tv_sec * 1000LL + (long long)ts.tv_nsec / 1000000LL;
}

void oo_sleep_ms(long long cap, long long ms) {
  struct timespec ts;
  oo_cap_require_time(cap, "sleep_ms");
  if (ms < 0) ms = 0;
  ts.tv_sec = (time_t)(ms / 1000);
  ts.tv_nsec = (long)((ms % 1000) * 1000000L);
  nanosleep(&ts, NULL);
}

long long oo_random(long long cap) {
  unsigned char b[8];
  long long v = 0;
  size_t i;
  oo_cap_require_rand(cap, "random");
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) == 0) {
    for (i = 0; i < 8; i++) v = (v << 8) | (long long)b[i];
    return v;
  }
#endif
  g_prng = g_prng * 6364136223846793005ULL + 1ULL;
  return (long long)(g_prng >> 1);
}

void oo_seed(long long cap, long long s) {
  oo_cap_require_rand(cap, "seed");
  g_prng = (unsigned long long)s | 1ULL;
}
