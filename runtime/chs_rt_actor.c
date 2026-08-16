/* M165 path A: thin actors under ThreadCap.
 * actor_spawn → joinable pthread + private mailbox; Ok("actor:N").
 * actor_send / actor_recv are non-blocking mailbox wrappers on that slot.
 * Not full actor model, supervision trees, or multi-process. */
#include "chs_rt.h"
#include <pthread.h>
#include <unistd.h>

#define OO_ACTOR_SLOTS 16
#define OO_ACTOR_QDEPTH 8

typedef struct {
  int live;
  pthread_t thr;
  int head;
  int tail;
  int count;
  OoStr msgs[OO_ACTOR_QDEPTH];
  pthread_mutex_t mu;
} OoActor;

static OoActor g_actors[OO_ACTOR_SLOTS];
static pthread_mutex_t g_act_boot = PTHREAD_MUTEX_INITIALIZER;

static void *oo_actor_noop(void *arg) {
  (void)arg;
  usleep(10000);
  return NULL;
}

static OoStr oo_act_copy(OoStr s) {
  OoStr r;
  long long n;
  if (!s.data || s.len <= 0) {
    return oo_str_lit("");
  }
  n = s.len;
  if (n > (1LL << 20)) n = 1LL << 20;
  r.len = n;
  r.data = oo_str_alloc_payload((size_t)n);
  memcpy(r.data, s.data, (size_t)n);
  return r;
}

static int act_alloc_slot(void) {
  int i;
  for (i = 0; i < OO_ACTOR_SLOTS; i++) {
    if (!g_actors[i].live) return i;
  }
  return -1;
}

OoResS oo_actor_spawn(long long cap, OoStr name) {
  OoResS r;
  int slot;
  char buf[32];
  oo_cap_require_thread(cap, "actor_spawn");
  (void)name;
  pthread_mutex_lock(&g_act_boot);
  slot = act_alloc_slot();
  if (slot < 0) {
    pthread_mutex_unlock(&g_act_boot);
    r.ok = 0;
    r.val = oo_str_lit("actor_spawn: no free slot");
    return r;
  }
  g_actors[slot].live = 1;
  g_actors[slot].head = 0;
  g_actors[slot].tail = 0;
  g_actors[slot].count = 0;
  pthread_mutex_init(&g_actors[slot].mu, NULL);
  if (pthread_create(&g_actors[slot].thr, NULL, oo_actor_noop, NULL) != 0) {
    g_actors[slot].live = 0;
    pthread_mutex_destroy(&g_actors[slot].mu);
    pthread_mutex_unlock(&g_act_boot);
    r.ok = 0;
    r.val = oo_str_lit("actor_spawn failed");
    return r;
  }
  pthread_mutex_unlock(&g_act_boot);
  snprintf(buf, sizeof buf, "actor:%d", slot);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}

OoResS oo_actor_send(long long cap, long long id, OoStr msg) {
  OoResS r;
  int s = (int)id;
  OoActor *a;
  oo_cap_require_thread(cap, "actor_send");
  if (s < 0 || s >= OO_ACTOR_SLOTS) {
    r.ok = 0;
    r.val = oo_str_lit("actor_send: bad id");
    return r;
  }
  a = &g_actors[s];
  if (!a->live) {
    r.ok = 0;
    r.val = oo_str_lit("actor_send: empty slot");
    return r;
  }
  pthread_mutex_lock(&a->mu);
  if (a->count >= OO_ACTOR_QDEPTH) {
    pthread_mutex_unlock(&a->mu);
    r.ok = 0;
    r.val = oo_str_lit("actor_send: full");
    return r;
  }
  a->msgs[a->tail] = oo_act_copy(msg);
  a->tail = (a->tail + 1) % OO_ACTOR_QDEPTH;
  a->count++;
  pthread_mutex_unlock(&a->mu);
  r.ok = 1;
  r.val = oo_str_lit("sent");
  return r;
}

/* Non-blocking recv: Ok(msg) or Err empty (path A). */
OoResS oo_actor_recv(long long cap, long long id) {
  OoResS r;
  int s = (int)id;
  OoActor *a;
  oo_cap_require_thread(cap, "actor_recv");
  if (s < 0 || s >= OO_ACTOR_SLOTS) {
    r.ok = 0;
    r.val = oo_str_lit("actor_recv: bad id");
    return r;
  }
  a = &g_actors[s];
  if (!a->live) {
    r.ok = 0;
    r.val = oo_str_lit("actor_recv: empty slot");
    return r;
  }
  pthread_mutex_lock(&a->mu);
  if (a->count <= 0) {
    pthread_mutex_unlock(&a->mu);
    r.ok = 0;
    r.val = oo_str_lit("actor_recv: empty");
    return r;
  }
  r.ok = 1;
  r.val = a->msgs[a->head];
  a->head = (a->head + 1) % OO_ACTOR_QDEPTH;
  a->count--;
  pthread_mutex_unlock(&a->mu);
  return r;
}

/* OTP floor: join the noop thread and spawn a fresh one. Mailbox stays. */
OoResS oo_actor_restart(long long cap, long long id) {
  OoResS r;
  int s = (int)id;
  OoActor *a;
  oo_cap_require_thread(cap, "actor_restart");
  r.ok = 0;
  r.val = oo_str_lit("actor_restart failed");
  if (s < 0 || s >= OO_ACTOR_SLOTS) {
    r.val = oo_str_lit("actor_restart: bad id");
    return r;
  }
  a = &g_actors[s];
  pthread_mutex_lock(&g_act_boot);
  if (!a->live) {
    pthread_mutex_unlock(&g_act_boot);
    r.val = oo_str_lit("actor_restart: empty slot");
    return r;
  }
  pthread_join(a->thr, NULL);
  if (pthread_create(&a->thr, NULL, oo_actor_noop, NULL) != 0) {
    pthread_mutex_unlock(&g_act_boot);
    r.val = oo_str_lit("actor_restart: spawn failed");
    return r;
  }
  pthread_mutex_unlock(&g_act_boot);
  r.ok = 1;
  r.val = oo_str_lit("restarted");
  return r;
}

static unsigned char g_otp_once[OO_ACTOR_SLOTS];

OoResS oo_otp_supervise(long long cap, long long id) {
  OoResS r;
  int s = (int)id;
  oo_cap_require_thread(cap, "otp_supervise");
  r.ok = 0;
  r.val = oo_str_lit("otp_supervise: bad id");
  if (s < 0 || s >= OO_ACTOR_SLOTS) return r;
  if (g_otp_once[s]) {
    r.val = oo_str_lit("otp_supervise: already");
    return r;
  }
  if (!g_actors[s].live) {
    r.val = oo_str_lit("otp_supervise: empty");
    return r;
  }
  g_otp_once[s] = 1;
  return oo_actor_restart(cap, id);
}

static OoStr oo_rpc_mac(long long cap, OoStr payload) {
  char key[32];
  snprintf(key, sizeof key, "%llx", (unsigned long long)cap);
  return crypto_hmac_sha256_internal(oo_str_lit(key), payload);
}

OoResS oo_cap_rpc_send(long long cap, OoStr payload) {
  OoResS r;
  OoStr mac;
  char *out;
  oo_cap_require_thread(cap, "cap_rpc_send");
  r.ok = 0;
  r.val = oo_str_lit("cap_rpc_send: bad payload");
  if (payload.len < 0 || payload.len > 192) return r;
  mac = oo_rpc_mac(cap, payload);
  if (!mac.data || mac.len != 64) return r;
  out = oo_str_alloc_payload((size_t)(64 + payload.len));
  memcpy(out, mac.data, 64);
  if (payload.len > 0 && payload.data)
    memcpy(out + 64, payload.data, (size_t)payload.len);
  r.ok = 1;
  r.val.data = out;
  r.val.len = 64 + payload.len;
  return r;
}

OoResS oo_cap_rpc_recv(long long cap, OoStr sealed) {
  OoResS r;
  OoStr pay, mac;
  char *out;
  oo_cap_require_thread(cap, "cap_rpc_recv");
  r.ok = 0;
  r.val = oo_str_lit("cap_rpc_recv: hmac");
  if (!sealed.data || sealed.len < 64) return r;
  pay.data = sealed.data + 64;
  pay.len = sealed.len - 64;
  mac = oo_rpc_mac(cap, pay);
  if (mac.len != 64 || memcmp(mac.data, sealed.data, 64) != 0) return r;
  out = oo_str_alloc_payload((size_t)pay.len);
  if (pay.len > 0) memcpy(out, pay.data, (size_t)pay.len);
  r.ok = 1;
  r.val.data = out;
  r.val.len = pay.len;
  return r;
}
