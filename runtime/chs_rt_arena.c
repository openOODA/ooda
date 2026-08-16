/* Scoped bump arenas under ArenaCap (AllocCap still accepted). Reset is O(1). */
#include "chs_rt.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#define OO_ARENA_SLOTS 8

typedef struct {
  int live;
  char *base;
  size_t cap;
  size_t off;
} OoArena;

static OoArena g_ar[OO_ARENA_SLOTS];

static int ar_alloc_slot(void) {
  int i;
  for (i = 0; i < OO_ARENA_SLOTS; i++) {
    if (!g_ar[i].live) return i;
  }
  return -1;
}

static void oo_arena_need(long long cap, const char *op) {
  if (oo_cap_is_arena(cap) || oo_cap_is_alloc(cap)) return;
  fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n", op ? op : "arena");
  exit(1);
}

#define OO_CK_MAX 8
static long long g_ck[OO_CK_MAX];
static int g_ck_n;

OoResS oo_arena_create(long long cap, long long bytes) {
  OoResS r;
  int s;
  oo_arena_need(cap, "arena_create");
  r.ok = 0;
  r.val = oo_str_lit("arena_create failed");
  if (bytes < 64 || bytes > (1LL << 28)) {
    r.val = oo_str_lit("arena_create: bad size");
    return r;
  }
  s = ar_alloc_slot();
  if (s < 0) {
    r.val = oo_str_lit("arena_create: no slot");
    return r;
  }
  g_ar[s].base = (char *)malloc((size_t)bytes);
  if (!g_ar[s].base) {
    r.val = oo_str_lit("arena_create: oom");
    return r;
  }
  g_ar[s].live = 1;
  g_ar[s].cap = (size_t)bytes;
  g_ar[s].off = 0;
  {
    char buf[32];
    snprintf(buf, sizeof buf, "arena:%d", s);
    r.ok = 1;
    r.val = oo_str_lit(buf);
  }
  return r;
}

OoResS oo_arena_alloc(long long cap, long long id, long long n) {
  OoResS r;
  int s = (int)id;
  OoArena *a;
  oo_arena_need(cap, "arena_alloc");
  r.ok = 0;
  r.val = oo_str_lit("arena_alloc failed");
  if (s < 0 || s >= OO_ARENA_SLOTS || !g_ar[s].live) {
    r.val = oo_str_lit("arena_alloc: bad id");
    return r;
  }
  if (n <= 0 || n > (1LL << 26)) {
    r.val = oo_str_lit("arena_alloc: bad n");
    return r;
  }
  a = &g_ar[s];
  if (a->off + (size_t)n > a->cap) {
    r.val = oo_str_lit("arena_alloc: full");
    return r;
  }
  {
    char buf[32];
    snprintf(buf, sizeof buf, "%llu", (unsigned long long)a->off);
    a->off += (size_t)n;
    r.ok = 1;
    r.val = oo_str_lit(buf);
  }
  return r;
}

OoResS oo_arena_reset(long long cap, long long id) {
  OoResS r;
  int s = (int)id;
  oo_arena_need(cap, "arena_reset");
  r.ok = 0;
  r.val = oo_str_lit("arena_reset failed");
  if (s < 0 || s >= OO_ARENA_SLOTS || !g_ar[s].live) {
    r.val = oo_str_lit("arena_reset: bad id");
    return r;
  }
  g_ar[s].off = 0;
  r.ok = 1;
  r.val = oo_str_lit("OK");
  return r;
}

long long oo_soa_layout(OoStr name) {
  if (!name.data || name.len <= 0) return 0;
  return 1;
}

long long oo_dod_layout(long long n) {
  if (n < 0) return 0;
  return n;
}

long long oo_checkpoint(long long v) {
  if (g_ck_n >= OO_CK_MAX) return -1;
  g_ck[g_ck_n] = v;
  return (long long)g_ck_n++;
}

long long oo_rollback(void) {
  if (g_ck_n <= 0) return 0;
  return g_ck[--g_ck_n];
}
