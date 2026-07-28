/* CHS runtime for stage-0 C backend (native stage-1 without clang). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

typedef struct {
  char *data;
  long long len;
} OoStr;

typedef struct {
  long long *data;
  long long len;
  long long cap;
} OoIList;

typedef struct {
  OoStr *data;
  long long len;
  long long cap;
} OoSList;

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
typedef struct {
  int ok;
  OoStr val;
} OoResS;

OoResS oo_read_file(OoStr path) {
  OoResS r;
  FILE *f = fopen(path.data, "rb");
  if (!f) {
    r.ok = 0;
    r.val = oo_str_lit("read_file failed");
    return r;
  }
  fseek(f, 0, SEEK_END);
  long sz = ftell(f);
  fseek(f, 0, SEEK_SET);
  char *buf = (char *)malloc((size_t)sz + 1);
  if (!buf) abort();
  size_t n = fread(buf, 1, (size_t)sz, f);
  buf[n] = 0;
  fclose(f);
  r.ok = 1;
  r.val.data = buf;
  r.val.len = (long long)n;
  return r;
}

typedef struct {
  int ok;
  OoStr err;
} OoResV;

OoResV oo_write_file(OoStr path, OoStr content) {
  OoResV r;
  FILE *f = fopen(path.data, "wb");
  if (!f) {
    r.ok = 0;
    r.err = oo_str_lit("write_file failed");
    return r;
  }
  fwrite(content.data, 1, (size_t)content.len, f);
  fclose(f);
  r.ok = 1;
  r.err = oo_str_lit("");
  return r;
}

void oo_print_str(OoStr s) { fwrite(s.data, 1, (size_t)s.len, stdout); }
void oo_print_int(long long n) { printf("%lld", n); }
void oo_print_bool(int b) { fputs(b ? "true" : "false", stdout); }
void oo_println(void) { fputc('\n', stdout); }

int oo_str_eq(OoStr a, OoStr b) {
  if (a.len != b.len) return 0;
  return memcmp(a.data, b.data, (size_t)a.len) == 0;
}
