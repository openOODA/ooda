#include "chs_rt.h"

char *oo_str_alloc_payload(size_t len) {
  OoStrHeader *hdr = (OoStrHeader *)malloc(sizeof(OoStrHeader) + len + 1);
  if (!hdr) abort();
  hdr->ref_count = 1;
  hdr->flags = 0;
  char *data = (char *)(hdr + 1);
  data[len] = 0;
  return data;
}

static int oo_str_hdr_ok(OoStr s) {
  if (!s.data) return 0;
  if (s.len < 0 || s.len > (1LL << 28)) return 0;
  /* Payload must be after a heap header we allocated. */
  if (((uintptr_t)s.data) < sizeof(OoStrHeader) + 8) return 0;
  OoStrHeader *hdr = ((OoStrHeader *)s.data) - 1;
  if (hdr->ref_count == 0 || hdr->ref_count == UINT32_MAX) return 0;
  if (hdr->ref_count > 1000000u) return 0;
  if (hdr->flags & OO_FLAG_STATIC) return 0;
  return 1;
}

void oo_str_retain(OoStr s) {
  if (!oo_str_hdr_ok(s)) return;
  OoStrHeader *hdr = ((OoStrHeader *)s.data) - 1;
  hdr->ref_count++;
}

void oo_str_release(OoStr s) {
  if (!oo_str_hdr_ok(s)) return;
  OoStrHeader *hdr = ((OoStrHeader *)s.data) - 1;
  if (hdr->ref_count > 0) {
    hdr->ref_count--;
    /* Do not free: seed-era emit still over-releases / use-after-free.
       Leaking is preferred to heap corruption until emit ARC is complete. */
    (void)hdr;
  }
}

OoStr oo_str_lit(const char *s) {
  OoStr r;
  if (!s) {
    r.len = 0;
    r.data = oo_str_alloc_payload(0);
    return r;
  }
  r.len = (long long)strlen(s);
  r.data = oo_str_alloc_payload((size_t)r.len);
  memcpy(r.data, s, (size_t)r.len);
  return r;
}

/* Non-consuming concat: borrows a/b (M2: s=s+t safe with reassign_arc). */
OoStr oo_str_concat(OoStr a, OoStr b) {
  OoStr r;
  long long al = (a.data && a.len > 0 && a.len < (1LL << 28)) ? a.len : 0;
  long long bl = (b.data && b.len > 0 && b.len < (1LL << 28)) ? b.len : 0;
  r.len = al + bl;
  r.data = oo_str_alloc_payload((size_t)r.len);
  if (al > 0) {
    memcpy(r.data, a.data, (size_t)al);
  }
  if (bl > 0) {
    memcpy(r.data + al, b.data, (size_t)bl);
  }
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
  r.data = oo_str_alloc_payload((size_t)nbytes);
  memcpy(r.data, s.data + b, (size_t)nbytes);
  return r;
}

OoStr oo_str_slice(OoStr s, long long start, long long end) {
  long long bs = utf8_byte_index(s, start);
  long long be = (end == oo_chars_len(s)) ? s.len : utf8_byte_index(s, end);
  if (bs < 0 || be < 0 || be < bs) {
    /* Fail soft for bootstrap emit edge cases (empty field / OOB) — empty string. */
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  OoStr r;
  r.len = be - bs;
  r.data = oo_str_alloc_payload((size_t)r.len);
  memcpy(r.data, s.data + bs, (size_t)r.len);
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
