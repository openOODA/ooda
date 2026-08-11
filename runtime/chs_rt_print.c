#include "chs_rt.h"

void oo_print_str(OoStr s) { fwrite(s.data, 1, (size_t)s.len, stdout); }
void oo_eprint_str(OoStr s) { fwrite(s.data, 1, (size_t)s.len, stderr); }
void oo_print_int(long long n) { printf("%lld", n); }
void oo_print_bool(int b) { fputs(b ? "true" : "false", stdout); }
void oo_println(void) { fputc('\n', stdout); }
void oo_eprintln(void) { fputc('\n', stderr); }

int oo_str_eq(OoStr a, OoStr b) {
  if (a.len != b.len) return 0;
  return memcmp(a.data, b.data, (size_t)a.len) == 0;
}

int oo_str_contains(OoStr hay, OoStr needle) {
  if (needle.len == 0) return 1;
  if (needle.len > hay.len) return 0;
  /* Length-bounded search so zero-copy slices (no interior NUL) stay correct. */
  for (long long i = 0; i + needle.len <= hay.len; i++) {
    if (memcmp(hay.data + i, needle.data, (size_t)needle.len) == 0) return 1;
  }
  return 0;
}
OoStr oo_int_to_str(long long n) {
  /* Fixed stack buffer then one owned heap copy of exact printed length. */
  char buf[32];
  int nwritten = snprintf(buf, sizeof(buf), "%lld", n);
  if (nwritten < 0) abort();
  return oo_str_lit(buf);
}

OoStr oo_str_trim(OoStr s) {
  if (!s.data || s.len == 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  long long start = 0;
  while (start < s.len && isspace((unsigned char)s.data[start])) start++;
  long long end = s.len;
  while (end > start && isspace((unsigned char)s.data[end - 1])) end--;
  long long tlen = end - start;
  if (tlen <= 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  OoStr r;
  r.len = tlen;
  r.data = oo_str_alloc_payload((size_t)tlen);
  memcpy(r.data, s.data + start, (size_t)tlen);
  return r;
}

OoStr oo_str_to_lowercase(OoStr s) {
  if (!s.data || s.len == 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  int needs = 0;
  for (long long i = 0; i < s.len; i++) {
    unsigned char c = (unsigned char)s.data[i];
    if (c >= 'A' && c <= 'Z') {
      needs = 1;
      break;
    }
  }
  if (!needs) {
    oo_str_retain(s);
    return s;
  }
  OoStr r;
  r.len = s.len;
  r.data = oo_str_alloc_payload((size_t)r.len);
  for (long long i = 0; i < s.len; i++) {
    r.data[i] = (char)tolower((unsigned char)s.data[i]);
  }
  return r;
}

/* Path A M165 free name: owned ASCII upper; not &str. Method: to_lowercase exists. */
OoStr oo_str_to_uppercase(OoStr s) {
  if (!s.data || s.len == 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  int needs = 0;
  for (long long i = 0; i < s.len; i++) {
    unsigned char c = (unsigned char)s.data[i];
    if (c >= 'a' && c <= 'z') {
      needs = 1;
      break;
    }
  }
  if (!needs) {
    oo_str_retain(s);
    return s;
  }
  OoStr r;
  r.len = s.len;
  r.data = oo_str_alloc_payload((size_t)r.len);
  for (long long i = 0; i < s.len; i++) {
    r.data[i] = (char)toupper((unsigned char)s.data[i]);
  }
  return r;
}
