#include "chs_rt.h"

void *oo_list_alloc_payload(size_t elem_size, size_t cap) {
  if (cap == 0) return NULL;
  OoListHeader *hdr = (OoListHeader *)malloc(sizeof(OoListHeader) + cap * elem_size);
  if (!hdr) abort();
  hdr->ref_count = 1;
  hdr->flags = 0;
  return (void *)(hdr + 1);
}

OoIList oo_ilist_new(void) {
  OoIList l = {NULL, 0, 0};
  return l;
}

void oo_ilist_retain(OoIList l) {
  if (!l.data) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  if (hdr->ref_count == 0 || hdr->ref_count == UINT32_MAX || (hdr->flags & 1)) return;
  hdr->ref_count++;
}

static int oo_list_hdr_ok(void *data, long long len, long long cap) {
  if (!data) return 0;
  if (len < 0 || cap < 0 || len > (1LL << 28) || cap > (1LL << 28)) return 0;
  if (cap > 0 && len > cap) return 0;
  if (((uintptr_t)data) < sizeof(OoListHeader) + 8) return 0;
  OoListHeader *hdr = ((OoListHeader *)data) - 1;
  if (hdr->ref_count == 0 || hdr->ref_count == UINT32_MAX) return 0;
  if (hdr->ref_count > 1000000u) return 0;
  if (hdr->flags & 1) return 0;
  return 1;
}

void oo_ilist_release(OoIList l) {
  if (!oo_list_hdr_ok(l.data, l.len, l.cap)) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  if (hdr->ref_count > 0) {
    hdr->ref_count--;
    /* Leak-not-free: see oo_str_release. */
    (void)hdr;
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
  return n;
}

long long oo_ilist_get(OoIList l, long long i) {
  if (i < 0 || i >= l.len) {
    fprintf(stderr, "ilist_get OOB\n");
    abort();
  }
  return l.data[i];
}

long long oo_ilist_len(OoIList l) { return l.len; }

OoSList oo_slist_new(void) {
  OoSList l = {NULL, 0, 0};
  return l;
}

void oo_slist_retain(OoSList l) {
  if (!l.data) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  if (hdr->ref_count == 0 || hdr->ref_count == UINT32_MAX || (hdr->flags & 1)) return;
  hdr->ref_count++;
}

void oo_slist_release(OoSList l) {
  if (!oo_list_hdr_ok(l.data, l.len, l.cap)) return;
  OoListHeader *hdr = ((OoListHeader *)l.data) - 1;
  if (hdr->ref_count > 0) {
    hdr->ref_count--;
    /* Leak-not-free: see oo_str_release. */
    (void)hdr;
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
  return n;
}

OoStr oo_slist_get(OoSList l, long long i) {
  if (i < 0 || i >= l.len) {
    fprintf(stderr, "slist_get OOB\n");
    abort();
  }
  return l.data[i];
}

long long oo_slist_len(OoSList l) { return l.len; }
