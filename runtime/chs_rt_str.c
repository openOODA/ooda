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
  if (hdr->flags == 0xFFFFFFFFu) return 0;
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
    if (hdr->ref_count == 0) {
      hdr->flags = 0xFFFFFFFFu;
      free(hdr);
    }
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

/* Path A Byte floor (M162): raw byte 0..255 at index; -1 if OOB. Not &str borrow. */
long long oo_byte_at(OoStr s, long long idx) {
  if (!s.data || idx < 0 || idx >= s.len) return -1;
  return (long long)(unsigned char)s.data[idx];
}

/* Path A (M163): byte length alias of oo_str_byte_len. Not UTF-8 char count. */
long long oo_bytes_len(OoStr s) { return oo_str_byte_len(s); }

/* Path A: owned copy of raw bytes [start,end). Not &str borrow / no lifetime. */
OoStr oo_byte_slice(OoStr s, long long start, long long end) {
  if (!s.data || s.len < 0) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  if (start < 0) start = 0;
  if (end > s.len) end = s.len;
  if (start > end || start >= s.len) {
    OoStr empty;
    empty.len = 0;
    empty.data = oo_str_alloc_payload(0);
    return empty;
  }
  OoStr r;
  r.len = end - start;
  r.data = oo_str_alloc_payload((size_t)r.len);
  memcpy(r.data, s.data + (size_t)start, (size_t)r.len);
  return r;
}

/* Path A: raw byte equality (len + memcmp). Not &str borrow. */
int oo_bytes_eq(OoStr a, OoStr b) { return oo_str_eq(a, b); }

/* Path A: owned identity — String as byte-string view (still OoStr, not &str). */
OoStr oo_bytes_from_str(OoStr s) {
  return oo_byte_slice(s, 0, oo_bytes_len(s));
}

/* Path A: raw byte concat (alias of oo_str_concat; not UTF-8 merge). */
OoStr oo_bytes_concat(OoStr a, OoStr b) { return oo_str_concat(a, b); }

/* Path A Byte buffer = List[Int] elements in 0..255. Not List[Byte] ABI. */
OoIList oo_bytes_new(void) { return oo_ilist_new(); }

OoIList oo_bytes_push(OoIList l, long long b) {
  if (b < 0) b = 0;
  if (b > 255) b = 255;
  return oo_ilist_push(l, b);
}

/* Soft OOB like byte_at: -1; else stored 0..255 Int. */
long long oo_bytes_get(OoIList l, long long i) {
  if (!l.data || i < 0 || i >= l.len) return -1;
  long long v = l.data[i];
  if (v < 0) return 0;
  if (v > 255) return 255;
  return v;
}

/* Build owned OoStr from Byte buffer (List[Int] 0..255). Clamps each elem. */
OoStr oo_bytes_to_str(OoIList l) {
  long long n = (l.data && l.len > 0 && l.len < (1LL << 28)) ? l.len : 0;
  OoStr r;
  r.len = n;
  r.data = oo_str_alloc_payload((size_t)n);
  for (long long i = 0; i < n; i++) {
    long long v = l.data[i];
    if (v < 0) v = 0;
    if (v > 255) v = 255;
    r.data[i] = (char)(unsigned char)v;
  }
  return r;
}

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
