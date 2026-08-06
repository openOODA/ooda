#include "chs_rt.h"

/* Capability tokens (must match emit preamble + main inject). */
#ifndef OO_CAP_FS
#define OO_CAP_FS  0x4F4F4653LL /* OOFS */
#define OO_CAP_SYS 0x4F4F5359LL /* OOSY */
#define OO_CAP_ENV 0x4F4F454ELL /* OOEN */
#define OO_CAP_NET 0x4F4F4E54LL /* OONT */
#endif

static void oo_cap_require(long long got, long long want, const char *op) {
  if (got != want) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability (static+runtime seal)\n", op);
    exit(1);
  }
}

OoResS oo_read_file(long long cap, OoStr path) {
  oo_cap_require(cap, OO_CAP_FS, "read_file");
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

OoResV oo_write_file(long long cap, OoStr path, OoStr content) {
  oo_cap_require(cap, OO_CAP_FS, "write_file");
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

int oo_path_exists(long long cap, OoStr path) {
  oo_cap_require(cap, OO_CAP_FS, "path_exists");
  FILE *f = fopen(path.data, "rb");
  if (f) {
    fclose(f);
    return 1;
  }
  return 0;
}

long long oo_file_size(long long cap, OoStr path) {
  oo_cap_require(cap, OO_CAP_FS, "file_size");
  FILE *f = fopen(path.data, "rb");
  if (!f) return -1;
  fseek(f, 0, SEEK_END);
  long long sz = ftell(f);
  fclose(f);
  return sz;
}

OoResS oo_env_get(long long cap, OoStr key) {
  oo_cap_require(cap, OO_CAP_ENV, "env_get");
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
