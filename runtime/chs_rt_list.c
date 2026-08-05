#include "chs_rt.h"

OoIList oo_ilist_new(void) {
  OoIList l = {NULL, 0, 0};
  return l;
}

OoIList oo_ilist_push(OoIList l, long long v) {
  if (l.len + 1 > l.cap) {
    l.cap = l.cap ? l.cap * 2 : 8;
    l.data = (long long *)realloc(l.data, (size_t)l.cap * sizeof(long long));
    if (!l.data) abort();
  }
  l.data[l.len++] = v;
  return l;
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

OoSList oo_slist_push(OoSList l, OoStr v) {
  if (l.len + 1 > l.cap) {
    l.cap = l.cap ? l.cap * 2 : 8;
    l.data = (OoStr *)realloc(l.data, (size_t)l.cap * sizeof(OoStr));
    if (!l.data) abort();
  }
  l.data[l.len++] = v;
  return l;
}

OoStr oo_slist_get(OoSList l, long long i) {
  if (i < 0 || i >= l.len) {
    fprintf(stderr, "slist_get OOB\n");
    abort();
  }
  return l.data[i];
}

long long oo_slist_len(OoSList l) { return l.len; }

/* Result[String,String] as {ok, val} where ok=1 means Ok */
