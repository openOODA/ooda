/* M163: joinable pthread path A under ThreadCap (slot table, not detach). */
#include "chs_rt.h"
#include <unistd.h>
#include <pthread.h>

#define OO_THREAD_SLOTS 64
static pthread_t g_threads[OO_THREAD_SLOTS];
static int g_thread_live[OO_THREAD_SLOTS]; /* 1 = joinable live slot */
static pthread_mutex_t g_thr_boot = PTHREAD_MUTEX_INITIALIZER;

static void *oo_thread_noop(void *arg) {
  (void)arg;
  /* brief sleep so join has something real to wait on */
  usleep(10000);
  return NULL;
}

static int thr_alloc_slot(void) {
  int i;
  for (i = 0; i < OO_THREAD_SLOTS; i++) {
    if (!g_thread_live[i]) return i;
  }
  return -1;
}

/* Parse "tid:N" → slot, or -1 on failure. */
static long long thr_parse_tid(OoStr s) {
  long long n = 0;
  long long i;
  if (!s.data || s.len < 5) return -1;
  if (s.data[0] != 't' || s.data[1] != 'i' || s.data[2] != 'd' || s.data[3] != ':')
    return -1;
  if (s.len == 4) return -1;
  for (i = 4; i < s.len; i++) {
    char c = s.data[i];
    if (c < '0' || c > '9') return -1;
    n = n * 10 + (long long)(c - '0');
    if (n >= OO_THREAD_SLOTS) return -1;
  }
  return n;
}

OoResS oo_thread_spawn(long long cap, OoStr name) {
  OoResS r;
  int slot;
  char buf[32];
  oo_cap_require_thread(cap, "thread_spawn");
  (void)name;
  pthread_mutex_lock(&g_thr_boot);
  slot = thr_alloc_slot();
  if (slot < 0) {
    pthread_mutex_unlock(&g_thr_boot);
    r.ok = 0;
    r.val = oo_str_lit("thread_spawn: no free slot");
    return r;
  }
  if (pthread_create(&g_threads[slot], NULL, oo_thread_noop, NULL) != 0) {
    pthread_mutex_unlock(&g_thr_boot);
    r.ok = 0;
    r.val = oo_str_lit("thread_spawn failed");
    return r;
  }
  g_thread_live[slot] = 1;
  pthread_mutex_unlock(&g_thr_boot);
  snprintf(buf, sizeof buf, "tid:%d", slot);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}

/* Preferred: join by Int slot index. */
OoResS oo_thread_join(long long cap, long long slot) {
  OoResS r;
  int s = (int)slot;
  pthread_t th;
  oo_cap_require_thread(cap, "thread_join");
  if (s < 0 || s >= OO_THREAD_SLOTS) {
    r.ok = 0;
    r.val = oo_str_lit("thread_join: bad slot");
    return r;
  }
  pthread_mutex_lock(&g_thr_boot);
  if (!g_thread_live[s]) {
    pthread_mutex_unlock(&g_thr_boot);
    r.ok = 0;
    r.val = oo_str_lit("thread_join: empty slot");
    return r;
  }
  th = g_threads[s];
  g_thread_live[s] = 0;
  pthread_mutex_unlock(&g_thr_boot);
  if (pthread_join(th, NULL) != 0) {
    r.ok = 0;
    r.val = oo_str_lit("thread_join failed");
    return r;
  }
  r.ok = 1;
  r.val = oo_str_lit("joined");
  return r;
}

/* Join by String "tid:N" (parse → slot). */
OoResS oo_thread_join_s(long long cap, OoStr tid) {
  long long slot = thr_parse_tid(tid);
  OoResS r;
  if (slot < 0) {
    oo_cap_require_thread(cap, "thread_join");
    r.ok = 0;
    r.val = oo_str_lit("thread_join: bad tid");
    return r;
  }
  return oo_thread_join(cap, slot);
}
