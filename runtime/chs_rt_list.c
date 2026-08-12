#include "chs_rt.h"
#include <pthread.h>

/* CHANGE B (cont.): ambient-quota counter mutex. Defined here because the
 * state (oo_list_ambient_quota / oo_list_ambient_bytes) lives in this TU.
 * chs_rt_alloc.c uses it under `extern pthread_mutex_t g_quota_mu`. */
pthread_mutex_t g_quota_mu = PTHREAD_MUTEX_INITIALIZER;

/* Default ambient budget for list payloads (not OS rlimit). Override via
 * OO_LIST_AMBIENT_QUOTA (bytes, positive). alloc_bytes raises the ceiling. */
long long oo_list_ambient_quota = 64LL * 1024 * 1024;
long long oo_list_ambient_bytes = 0;

/* HIGH-1: pthread_once mirrors g_alloc_once in chs_rt_alloc.c. The body
 * runs exactly once process-wide; concurrent callers block until done. */
static pthread_once_t g_quota_once = PTHREAD_ONCE_INIT;

static void oo_list_quota_init_once(void) {
  /* ZT: only process-policy key OO_LIST_AMBIENT_QUOTA (not arbitrary getenv) */
  const char *e = oo_process_policy_getenv("OO_LIST_AMBIENT_QUOTA");
  if (e && e[0]) {
    long long v = atoll(e);
    if (v > 0) oo_list_ambient_quota = v;
  }
}

void oo_list_quota_init_public(void) {
  pthread_once(&g_quota_once, oo_list_quota_init_once);
}

void *oo_list_alloc_payload(size_t elem_size, size_t cap) {
  if (cap == 0) return NULL;
  size_t bytes = sizeof(OoListHeader) + cap * elem_size;
  oo_list_quota_init_public();
  pthread_mutex_lock(&g_quota_mu);
  if (oo_list_ambient_bytes + (long long)bytes > oo_list_ambient_quota) {
    pthread_mutex_unlock(&g_quota_mu);
    fprintf(stderr, "ERR\tcap\tambient List memory quota exceeded (AllocCap required)\n");
    exit(1);
  }
  oo_list_ambient_bytes += (long long)bytes;
  pthread_mutex_unlock(&g_quota_mu);
  OoListHeader *hdr = (OoListHeader *)malloc(bytes);
  if (!hdr) abort();
  /* CHANGE C: ref_count / flags are accessed concurrently by retain /
   * release across threads. Use __atomic_* GCC intrinsics on the plain
   * uint32_t fields (no struct-type change required).
   * CRIT-2 contract: oo_list_alloc_payload leaves ref_count=0; the caller
   * is responsible for initialising the payload and then atomically
   * publishing it by storing ref_count=1 with __ATOMIC_RELEASE. A racing
   * retainer must observe ref_count==0 (or 1 after publish) and either
   * skip or read fully initialised slot data. */
  __atomic_store_n(&hdr->ref_count, 0, __ATOMIC_RELEASE);
  __atomic_store_n(&hdr->flags, 0, __ATOMIC_RELEASE);
  return (void *)(hdr + 1);
}

OoIList oo_ilist_new(void) {
  OoIList l = {NULL, 0, 0};
  return l;
}

void oo_ilist_retain(OoIList l) {
  if (!l.data) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  uint32_t rc = __atomic_load_n(&hdr->ref_count, __ATOMIC_ACQUIRE);
  uint32_t fl = __atomic_load_n(&hdr->flags, __ATOMIC_ACQUIRE);
  if (rc == 0 || rc == UINT32_MAX || (fl & 1)) return;
  /* CAS loop so a concurrent release-to-zero cannot be lost. */
  while (rc > 0 && rc < UINT32_MAX) {
    if (__atomic_compare_exchange_n(&hdr->ref_count, &rc, rc + 1, 1,
                                    __ATOMIC_ACQ_REL, __ATOMIC_RELAXED)) {
      return;
    }
    rc = __atomic_load_n(&hdr->ref_count, __ATOMIC_RELAXED);
    fl = __atomic_load_n(&hdr->flags, __ATOMIC_RELAXED);
    if (rc == 0 || rc == UINT32_MAX || (fl & 1)) return;
  }
}

static int oo_list_hdr_ok(void *data, long long len, long long cap) {
  if (!data) return 0;
  if (len < 0 || cap < 0 || len > (1LL << 28) || cap > (1LL << 28)) return 0;
  if (cap > 0 && len > cap) return 0;
  if (((uintptr_t)data) < sizeof(OoListHeader) + 8) return 0;
  OoListHeader *hdr = ((OoListHeader *)data) - 1;
  uint32_t rc = __atomic_load_n(&hdr->ref_count, __ATOMIC_ACQUIRE);
  if (rc == 0 || rc == UINT32_MAX) return 0;
  if (rc > 1000000u) return 0;
  if (__atomic_load_n(&hdr->flags, __ATOMIC_ACQUIRE) & 1) return 0;
  return 1;
}

void oo_ilist_release(OoIList l) {
  if (!oo_list_hdr_ok(l.data, l.len, l.cap)) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  uint32_t prev = __atomic_fetch_sub(&hdr->ref_count, 1, __ATOMIC_ACQ_REL);
  if (prev == 1) {
    __atomic_store_n(&hdr->flags, 0xFFFFFFFFu, __ATOMIC_RELEASE);
    /* CRIT-1: ARM/POWER can reorder free() before the flags store above.
     * An explicit release fence guarantees the tombstone is globally
     * visible before the slot is reclaimed, without paying for SEQ_CST. */
    __atomic_thread_fence(__ATOMIC_RELEASE);
    pthread_mutex_lock(&g_quota_mu);
    oo_list_ambient_bytes -= (long long)(sizeof(OoListHeader) + l.cap * sizeof(long long));
    pthread_mutex_unlock(&g_quota_mu);
    free(hdr);
  }
}

void oo_ilist_free(OoIList l) {
  oo_ilist_release(l);
}

OoIList oo_ilist_push(OoIList l, long long v) {
  OoIList n;
  long long ncap = l.cap ? l.cap : 8;
  while (ncap < l.len + 1) ncap *= 2;
  n.data = (long long *)oo_list_alloc_payload(sizeof(long long), (size_t)ncap);
  if (l.data && l.len > 0) {
    memcpy(n.data, l.data, (size_t)l.len * sizeof(long long));
  }
  n.data[l.len] = v;
  n.len = l.len + 1;
  n.cap = ncap;
  /* CRIT-2: publish the freshly populated slot by setting ref_count=1
   * with release ordering so concurrent retainers either skip (rc==0)
   * or observe fully initialised payload (rc==1 + acquire load). */
  {
    OoListHeader *hdr = ((OoListHeader *)n.data) - 1;
    __atomic_store_n(&hdr->ref_count, 1, __ATOMIC_RELEASE);
  }
  return n;
}

long long oo_ilist_get(OoIList l, long long i) {
  /* CHANGE D: retain a reference across the read so a concurrent
   * release-to-zero from another thread cannot free the buffer while
   * we index into l.data. Release once read is complete. */
  oo_ilist_retain(l);
  long long v;
  if (i < 0 || i >= l.len) {
    oo_ilist_release(l);
    fprintf(stderr, "ilist_get OOB\n");
    abort();
  }
  v = l.data[i];
  oo_ilist_release(l);
  return v;
}

long long oo_ilist_len(OoIList l) { return l.len; }

OoSList oo_slist_new(void) {
  OoSList l = {NULL, 0, 0};
  return l;
}

void oo_slist_retain(OoSList l) {
  if (!l.data) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  uint32_t rc = __atomic_load_n(&hdr->ref_count, __ATOMIC_ACQUIRE);
  uint32_t fl = __atomic_load_n(&hdr->flags, __ATOMIC_ACQUIRE);
  if (rc == 0 || rc == UINT32_MAX || (fl & 1)) return;
  while (rc > 0 && rc < UINT32_MAX) {
    if (__atomic_compare_exchange_n(&hdr->ref_count, &rc, rc + 1, 1,
                                    __ATOMIC_ACQ_REL, __ATOMIC_RELAXED)) {
      return;
    }
    rc = __atomic_load_n(&hdr->ref_count, __ATOMIC_RELAXED);
    fl = __atomic_load_n(&hdr->flags, __ATOMIC_RELAXED);
    if (rc == 0 || rc == UINT32_MAX || (fl & 1)) return;
  }
}

void oo_slist_release(OoSList l) {
  if (!oo_list_hdr_ok(l.data, l.len, l.cap)) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  uint32_t prev = __atomic_fetch_sub(&hdr->ref_count, 1, __ATOMIC_ACQ_REL);
  if (prev == 1) {
    for (long long i = 0; i < l.len; i++) {
      oo_str_release(l.data[i]);
    }
    __atomic_store_n(&hdr->flags, 0xFFFFFFFFu, __ATOMIC_RELEASE);
    /* CRIT-1: see oo_ilist_release; same fence rationale. */
    __atomic_thread_fence(__ATOMIC_RELEASE);
    pthread_mutex_lock(&g_quota_mu);
    oo_list_ambient_bytes -= (long long)(sizeof(OoListHeader) + l.cap * sizeof(OoStr));
    pthread_mutex_unlock(&g_quota_mu);
    free(hdr);
  }
}

void oo_slist_free(OoSList l) {
  oo_slist_release(l);
}

OoSList oo_slist_push(OoSList l, OoStr v) {
  OoSList n;
  long long ncap = l.cap ? l.cap : 8;
  while (ncap < l.len + 1) ncap *= 2;
  n.data = (OoStr *)oo_list_alloc_payload(sizeof(OoStr), (size_t)ncap);
  if (l.data && l.len > 0) {
    memcpy(n.data, l.data, (size_t)l.len * sizeof(OoStr));
    for (long long i = 0; i < l.len; i++) {
      oo_str_retain(n.data[i]);
    }
  }
  n.data[l.len] = v;
  oo_str_retain(v);
  n.len = l.len + 1;
  n.cap = ncap;
  /* CRIT-2: publish the freshly populated slot by setting ref_count=1
   * with release ordering so concurrent retainers either skip (rc==0)
   * or observe fully initialised payload (rc==1 + acquire load). */
  {
    OoListHeader *hdr = ((OoListHeader *)n.data) - 1;
    __atomic_store_n(&hdr->ref_count, 1, __ATOMIC_RELEASE);
  }
  return n;
}

OoStr oo_slist_get(OoSList l, long long i) {
  /* CHANGE D: hold a list-level retain across the read so the underlying
   * payload cannot be freed mid-index by a concurrent release. */
  oo_slist_retain(l);
  OoStr r;
  if (i < 0 || i >= l.len) {
    oo_slist_release(l);
    fprintf(stderr, "slist_get OOB\n");
    abort();
  }
  /* Return an owned ref so let s = list_get(...) is free-safe. */
  oo_str_retain(l.data[i]);
  r = l.data[i];
  oo_slist_release(l);
  return r;
}

long long oo_slist_len(OoSList l) { return l.len; }
