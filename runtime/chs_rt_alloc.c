/* M17: process-local AllocCap — explicit alloc helpers only.
 * Not OS rlimit / heap isolation / ASAN.
 * Ambient List growth is quota-bounded (chs_rt_list); alloc_bytes raises ceiling. */
#include "chs_rt.h"
#include <unistd.h>
#include <pthread.h>
#include <limits.h>
#if defined(__linux__)
#include <sys/random.h>
#endif

/* CHANGE A: pthread_once replaces ad-hoc g_alloc_ready guard. Eliminates
 * the init race between threads calling oo_cap_grant_alloc /
 * oo_cap_require_alloc concurrently. */
static pthread_once_t g_alloc_once = PTHREAD_ONCE_INIT;
static long long g_tok_alloc;

/* Classic forgeable magic OOAL — must never be the live token. */
#define OO_CLASSIC_ALLOC 0x4F4F414CLL

static void alloc_init_once(void) {
  unsigned char b[8];
  size_t i;
  unsigned long long acc;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)(uintptr_t)&g_tok_alloc;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  /* 0x7… band (fs=1 sys=2 env=3 net=4 time=5 rand=6 alloc=7) */
  g_tok_alloc = 0x7000000000000000LL
      | (long long)((((unsigned long long)b[0]) << 56) | (((unsigned long long)b[1]) << 48) | (((unsigned long long)b[2]) << 40) | (((unsigned long long)b[3]) << 32) | (((unsigned long long)b[4]) << 24) | (((unsigned long long)b[5]) << 16) | (((unsigned long long)b[6]) << 8) | ((unsigned long long)b[7]));
  if (g_tok_alloc == OO_CLASSIC_ALLOC) g_tok_alloc ^= 0x11111111LL;
}

static void oo_alloc_init(void) {
  pthread_once(&g_alloc_once, alloc_init_once);
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

int oo_cap_is_alloc(long long got) {
  oo_alloc_init();
  return got == g_tok_alloc;
}

/* CHANGE B: quota counter is shared process state. Wrap the read-modify-
 * write of oo_list_ambient_quota in a mutex so concurrent alloc_bytes /
 * free_bytes callers cannot lose updates. The mutex lives in chs_rt_list.c
 * (where the ambient-quota state is owned); declared extern here. */
extern pthread_mutex_t g_quota_mu;

/* Per-raise ceiling for oo_alloc_bytes (bytes). Related to ambient List quota
 * (default 64MiB in chs_rt_list; raised process-locally via this helper).
 * Residual: not OS rlimit / setrlimit / cgroup heap isolation — process-local
 * ambient counter only. Full OS heap policy remains DESIGN. */
#define OO_ALLOC_BYTES_MAX (1LL << 30) /* 1 GiB per raise */

/* Smoke-friendly: re-check cap then return n as opaque size token (not real mmap).
 * Raises ambient List ceiling after env init (so OO_LIST_AMBIENT_QUOTA is base).
 * n < 0 or n > OO_ALLOC_BYTES_MAX → clamp to 0 (no-op raise).
 * Quota add saturates at LLONG_MAX (no signed overflow wrap). */
long long oo_alloc_bytes(long long cap, long long n) {
  extern void oo_list_quota_init_public(void);
  oo_cap_require_alloc(cap, "alloc_bytes");
  if (n < 0) n = 0;
  if (n > OO_ALLOC_BYTES_MAX) n = 0; /* oversize: no-op raise (not OS rlimit) */
  oo_list_quota_init_public();
  pthread_mutex_lock(&g_quota_mu);
  /* Sprint 1.7: check before add; saturate at LLONG_MAX on overflow. */
  if (n > LLONG_MAX - oo_list_ambient_quota)
    oo_list_ambient_quota = LLONG_MAX;
  else
    oo_list_ambient_quota += n;
  pthread_mutex_unlock(&g_quota_mu);
  return n;
}

/* Smoke-friendly: re-check cap; free is a no-op by handle.
 * Negative p must not inflate ambient quota (Seventh queue CRIT).
 * Sprint 1.6: oversize free (p > quota) is a no-op reclaim — leave quota
 * unchanged. Only subtract when p <= quota (exact free-to-zero is allowed). */
void oo_free_bytes(long long cap, long long p) {
  oo_cap_require_alloc(cap, "free_bytes");
  if (p < 0) p = 0;
  pthread_mutex_lock(&g_quota_mu);
  if (p <= oo_list_ambient_quota) {
    oo_list_ambient_quota -= p;
  }
  /* else: p > quota → reject oversize free as no-op reclaim */
  pthread_mutex_unlock(&g_quota_mu);
}
