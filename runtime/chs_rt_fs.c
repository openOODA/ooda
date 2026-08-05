#include "chs_rt.h"

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

