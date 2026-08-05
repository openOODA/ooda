#include "chs_rt.h"

OoStr oo_str_lit(const char *s) {
  OoStr r;
  r.len = (long long)strlen(s);
  r.data = (char *)malloc((size_t)r.len + 1);
  if (!r.data) abort();
  memcpy(r.data, s, (size_t)r.len + 1);
  return r;
}

OoStr oo_str_concat(OoStr a, OoStr b) {
  OoStr r;
  r.len = a.len + b.len;
  r.data = (char *)malloc((size_t)r.len + 1);
  if (!r.data) abort();
  memcpy(r.data, a.data, (size_t)a.len);
  memcpy(r.data + a.len, b.data, (size_t)b.len);
  r.data[r.len] = 0;
  return r;
}

long long oo_str_byte_len(OoStr s) { return s.len; }

long long oo_chars_len(OoStr s) {
  /* UTF-8 scalar count (ASCII-fast path covers CHS corpus). */
  long long n = 0;
  for (long long i = 0; i < s.len;) {
    unsigned char c = (unsigned char)s.data[i];
    if (c < 0x80) i += 1;
    else if ((c & 0xE0) == 0xC0) i += 2;
    else if ((c & 0xF0) == 0xE0) i += 3;
    else i += 4;
    n++;
  }
  return n;
}

static long long utf8_byte_index(OoStr s, long long char_idx) {
  long long n = 0;
  for (long long i = 0; i < s.len;) {
    if (n == char_idx) return i;
    unsigned char c = (unsigned char)s.data[i];
    if (c < 0x80) i += 1;
    else if ((c & 0xE0) == 0xC0) i += 2;
    else if ((c & 0xF0) == 0xE0) i += 3;
    else i += 4;
    n++;
  }
  return -1;
}

OoStr oo_char_at(OoStr s, long long idx) {
  long long b = utf8_byte_index(s, idx);
  if (b < 0) {
    fprintf(stderr, "char_at OOB\n");
    abort();
  }
  unsigned char c = (unsigned char)s.data[b];
  int nbytes = 1;
  if (c >= 0xF0) nbytes = 4;
  else if (c >= 0xE0) nbytes = 3;
  else if (c >= 0xC0) nbytes = 2;
  OoStr r;
  r.len = nbytes;
  r.data = (char *)malloc((size_t)nbytes + 1);
  memcpy(r.data, s.data + b, (size_t)nbytes);
  r.data[nbytes] = 0;
  return r;
}

OoStr oo_str_slice(OoStr s, long long start, long long end) {
  long long bs = utf8_byte_index(s, start);
  long long be = (end == oo_chars_len(s)) ? s.len : utf8_byte_index(s, end);
  if (bs < 0 || be < 0 || be < bs) {
    fprintf(stderr, "str_slice bad range\n");
    abort();
  }
  OoStr r;
  r.len = be - bs;
  r.data = (char *)malloc((size_t)r.len + 1);
  memcpy(r.data, s.data + bs, (size_t)r.len);
  r.data[r.len] = 0;
  return r;
}

int oo_char_is_digit(OoStr s) {
  return s.len == 1 && isdigit((unsigned char)s.data[0]);
}
int oo_char_is_alpha(OoStr s) {
  return s.len == 1 && isalpha((unsigned char)s.data[0]);
}
int oo_char_is_space(OoStr s) {
  return s.len == 1 && isspace((unsigned char)s.data[0]);
}

