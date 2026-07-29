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

int oo_path_exists(OoStr path) {
  FILE *f = fopen(path.data, "rb");
  if (f) {
    fclose(f);
    return 1;
  }
  return 0;
}

long long oo_file_size(OoStr path) {
  FILE *f = fopen(path.data, "rb");
  if (!f) return -1;
  fseek(f, 0, SEEK_END);
  long long sz = ftell(f);
  fclose(f);
  return sz;
}

OoResS oo_env_get(OoStr key) {
  OoResS r;
  char *val = getenv(key.data);
  if (val) {
    r.ok = 1;
    r.val = oo_str_lit(val);
  } else {
    r.ok = 0;
    r.val = oo_str_lit("env var not set");
  }
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

int oo_str_contains(OoStr hay, OoStr needle) {
  if (needle.len == 0) return 1;
  if (needle.len > hay.len) return 0;
  /* null-terminated copies for strstr; OoStr data is always 0-terminated by constructors */
  return strstr(hay.data, needle.data) != NULL;
}
OoStr oo_int_to_str(long long n) {
  OoStr r;
  r.data = (char *)malloc(32);
  if (!r.data) abort();
  r.len = snprintf(r.data, 32, "%lld", n);
  return r;
}

OoStr oo_str_trim(OoStr s) {
  long long start = 0;
  while (start < s.len && isspace((unsigned char)s.data[start])) start++;
  long long end = s.len;
  while (end > start && isspace((unsigned char)s.data[end - 1])) end--;
  OoStr r;
  r.len = end - start;
  r.data = (char *)malloc((size_t)r.len + 1);
  if (!r.data) abort();
  memcpy(r.data, s.data + start, (size_t)r.len);
  r.data[r.len] = 0;
  return r;
}

OoStr oo_str_to_lowercase(OoStr s) {
  OoStr r;
  r.len = s.len;
  r.data = (char *)malloc((size_t)r.len + 1);
  if (!r.data) abort();
  for (long long i = 0; i < s.len; i++) {
    r.data[i] = (char)tolower((unsigned char)s.data[i]);
  }
  r.data[r.len] = 0;
  return r;
}

/* ----- Host FFI wrappers (symbols from libooda.a) ----- */
extern char *ooda_host_ast_dump(const char *path);
extern char *ooda_host_check(const char *path);
extern char *ooda_host_token_dump(const char *path);
extern int ooda_host_chs_build(const char *src, const char *out_bin);
extern void ooda_host_free(char *p);

static OoStr oo_from_c_heap(char *p) {
  if (!p) return oo_str_lit("ERR\thost\tnull\n");
  OoStr r = oo_str_lit(p);
  ooda_host_free(p);
  return r;
}

OoStr oo_host_ast_dump(OoStr path) {
  return oo_from_c_heap(ooda_host_ast_dump(path.data));
}
OoStr oo_host_check(OoStr path) {
  return oo_from_c_heap(ooda_host_check(path.data));
}
OoStr oo_host_token_dump(OoStr path) {
  return oo_from_c_heap(ooda_host_token_dump(path.data));
}

OoResS oo_chs_build(OoStr src, OoStr out_bin) {
  OoResS r;
  int rc = ooda_host_chs_build(src.data, out_bin.data);
  if (rc == 0) {
    r.ok = 1;
    r.val = out_bin;
  } else {
    r.ok = 0;
    r.val = oo_str_lit("chs_build failed");
  }
  return r;
}
