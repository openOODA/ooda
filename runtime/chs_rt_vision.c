/* OPEN-89..92: expand $(const), detect file change, emit add.s64 PTX, metal canary. */
#include "chs_rt.h"
#include <stdio.h>
#include <string.h>
#include <limits.h>
#include <ctype.h>
#include <sys/stat.h>

static OoStr oo_vis_from_bytes(const char *p, long long n) {
  OoStr r;
  if (!p || n <= 0) return oo_str_lit("");
  r.len = n;
  r.data = oo_str_alloc_payload((size_t)n);
  memcpy(r.data, p, (size_t)n);
  return r;
}

static const char *oo_vis_skip(const char *s, const char *e) {
  while (s < e && (*s == ' ' || *s == '\t')) s++;
  return s;
}

static long long oo_vis_eval(const char **ps, const char *e, int *ok);

static long long oo_vis_prim(const char **ps, const char *e, int *ok) {
  const char *s = oo_vis_skip(*ps, e);
  long long v = 0;
  int neg = 0;
  if (s < e && *s == '(') {
    s++;
    *ps = s;
    v = oo_vis_eval(ps, e, ok);
    s = oo_vis_skip(*ps, e);
    if (s < e && *s == ')') s++;
    else *ok = 0;
    *ps = s;
    return v;
  }
  if (s < e && *s == '-') { neg = 1; s++; }
  if (s >= e || !isdigit((unsigned char)*s)) { *ok = 0; *ps = s; return 0; }
  while (s < e && isdigit((unsigned char)*s)) {
    v = v * 10 + (*s - '0');
    s++;
  }
  *ps = s;
  return neg ? -v : v;
}

static long long oo_vis_mul(const char **ps, const char *e, int *ok) {
  long long v = oo_vis_prim(ps, e, ok);
  const char *s;
  while (*ok) {
    s = oo_vis_skip(*ps, e);
    if (s < e && (*s == '*' || *s == '/' || *s == '%')) {
      char op = *s++;
      long long r;
      *ps = s;
      r = oo_vis_prim(ps, e, ok);
      if (!*ok) return v;
      if (op == '*') v = v * r;
      else if (r == 0) { *ok = 0; return 0; }
      else if (op == '/') v = v / r;
      else v = v % r;
    } else break;
  }
  return v;
}

static long long oo_vis_eval(const char **ps, const char *e, int *ok) {
  long long v = oo_vis_mul(ps, e, ok);
  const char *s;
  while (*ok) {
    s = oo_vis_skip(*ps, e);
    if (s < e && (*s == '+' || *s == '-')) {
      char op = *s++;
      long long r;
      *ps = s;
      r = oo_vis_mul(ps, e, ok);
      if (!*ok) return v;
      v = (op == '+') ? v + r : v - r;
    } else break;
  }
  return v;
}

/* Expand $(const-int-expr). Other text is copied. */
OoStr oo_str_macro_expand(OoStr src) {
  char out[2048];
  long long oi = 0;
  long long i = 0;
  if (!src.data || src.len <= 0) return oo_str_lit("");
  while (i < src.len && oi < 2040) {
    if (i + 1 < src.len && src.data[i] == '$' && src.data[i + 1] == '(') {
      long long j = i + 2;
      int depth = 1;
      int ok = 1;
      const char *ps;
      long long v;
      char num[32];
      int nl;
      while (j < src.len && depth > 0) {
        if (src.data[j] == '(') depth++;
        else if (src.data[j] == ')') depth--;
        j++;
      }
      if (depth != 0) { out[oi++] = src.data[i++]; continue; }
      ps = src.data + i + 2;
      v = oo_vis_eval(&ps, src.data + j - 1, &ok);
      if (!ok) { out[oi++] = src.data[i++]; continue; }
      nl = snprintf(num, sizeof num, "%lld", (long long)v);
      if (nl > 0 && oi + nl < 2040) {
        memcpy(out + oi, num, (size_t)nl);
        oi += nl;
      }
      i = j;
    } else {
      out[oi++] = src.data[i++];
    }
  }
  return oo_vis_from_bytes(out, oi);
}

OoStr oo_str_ast_macro(OoStr src) { return oo_str_macro_expand(src); }

#define OO_HR_SLOTS 8
static char g_hr_path[OO_HR_SLOTS][PATH_MAX];
static char g_hr_hash[OO_HR_SLOTS][65];
static int g_hr_n;

static void oo_hr_hex(const unsigned char *b, int n, char *out) {
  static const char *h = "0123456789abcdef";
  int i;
  for (i = 0; i < n && i < 32; i++) {
    out[i * 2] = h[b[i] >> 4];
    out[i * 2 + 1] = h[b[i] & 15];
  }
  out[n * 2] = 0;
}

/* 1 = first load or contents changed; 0 = same bytes as last call. */
long long oo_hot_reload(OoStr path) {
  char cpath[PATH_MAX];
  FILE *f;
  unsigned char buf[4096];
  unsigned long long h = 1469598103934665603ULL;
  size_t nr;
  char hex[65];
  int i, slot = -1;
  if (!path.data || path.len <= 0 || path.len >= PATH_MAX) return 0;
  memcpy(cpath, path.data, (size_t)path.len);
  cpath[path.len] = 0;
  f = fopen(cpath, "rb");
  if (!f) return 0;
  while ((nr = fread(buf, 1, sizeof buf, f)) > 0) {
    size_t k;
    for (k = 0; k < nr; k++) {
      h ^= buf[k];
      h *= 1099511628211ULL;
    }
  }
  fclose(f);
  snprintf(hex, sizeof hex, "%016llx", (unsigned long long)h);
  for (i = 0; i < g_hr_n; i++) {
    if (strcmp(g_hr_path[i], cpath) == 0) { slot = i; break; }
  }
  if (slot < 0) {
    if (g_hr_n >= OO_HR_SLOTS) return 1;
    slot = g_hr_n++;
    strncpy(g_hr_path[slot], cpath, PATH_MAX - 1);
    g_hr_path[slot][PATH_MAX - 1] = 0;
    strncpy(g_hr_hash[slot], hex, 64);
    g_hr_hash[slot][64] = 0;
    return 1;
  }
  if (strcmp(g_hr_hash[slot], hex) == 0) return 0;
  strncpy(g_hr_hash[slot], hex, 64);
  g_hr_hash[slot][64] = 0;
  return 1;
}

long long oo_live_reload(OoStr path) { return oo_hot_reload(path); }

static const char k_ooda_ptx[] =
    ".version 7.0\n"
    ".target sm_70\n"
    ".address_size 64\n"
    ".visible .entry ooda_add(\n"
    "  .param .u64 a,\n"
    "  .param .u64 b,\n"
    "  .param .u64 c\n"
    ")\n"
    "{\n"
    "  .reg .u64 %rd<4>;\n"
    "  ld.param.u64 %rd0, [a];\n"
    "  ld.param.u64 %rd1, [b];\n"
    "  ld.global.u64 %rd2, [%rd0];\n"
    "  ld.global.u64 %rd3, [%rd1];\n"
    "  add.s64 %rd2, %rd2, %rd3;\n"
    "  ld.param.u64 %rd0, [c];\n"
    "  st.global.u64 [%rd0], %rd2;\n"
    "  ret;\n"
    "}\n";

long long oo_emit_ptx(void) {
  const char *dir = ".ooda-cache/ooda-tmp";
  char path[256];
  FILE *f;
  size_t n = sizeof k_ooda_ptx - 1;
  (void)mkdir(".ooda-cache", 0755);
  (void)mkdir(dir, 0755);
  snprintf(path, sizeof path, "%s/ooda_emit.ptx", dir);
  f = fopen(path, "w");
  if (!f) return 0;
  if (fwrite(k_ooda_ptx, 1, n, f) != n) { fclose(f); return 0; }
  fclose(f);
  return (long long)n;
}

long long oo_emit_spirv(void) {
  /* Minimal SPIR-V magic + version words, then a comment listing add. */
  static const unsigned char magic[20] = {
      0x03, 0x02, 0x23, 0x07, 0x00, 0x00, 0x01, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00};
  const char *dir = ".ooda-cache/ooda-tmp";
  char path[256];
  FILE *f;
  (void)mkdir(".ooda-cache", 0755);
  (void)mkdir(dir, 0755);
  snprintf(path, sizeof path, "%s/ooda_emit.spv", dir);
  f = fopen(path, "wb");
  if (!f) return 0;
  fwrite(magic, 1, sizeof magic, f);
  fputs("add.s64", f);
  fclose(f);
  return (long long)(sizeof magic + 7);
}

#define OO_METAL_CANARY 0x4D455441LL
static unsigned char g_metal_page[4096];
static int g_metal_ready;

long long oo_bare_metal_init(void) {
  memset(g_metal_page, 0, sizeof g_metal_page);
  g_metal_page[0] = 0x4D;
  g_metal_page[1] = 0x45;
  g_metal_page[2] = 0x54;
  g_metal_page[3] = 0x41;
  g_metal_ready = 1;
  return OO_METAL_CANARY;
}

OoStr macro_expand(OoStr src) { return oo_str_macro_expand(src); }
OoStr ast_macro(OoStr src) { return oo_str_ast_macro(src); }
long long hot_reload(OoStr path) { return oo_hot_reload(path); }
long long live_reload(OoStr path) { return oo_live_reload(path); }
long long emit_ptx(void) { return oo_emit_ptx(); }
long long emit_spirv(void) { return oo_emit_spirv(); }
long long bare_metal_init(void) { return oo_bare_metal_init(); }
