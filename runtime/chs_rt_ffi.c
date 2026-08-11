/* M156/M162: process-local UnsafeFFICap + optional allowlisted OS dlopen. */
#include "chs_rt.h"
#include <unistd.h>
#include <dlfcn.h>
#if defined(__linux__) || defined(__APPLE__)
#include <sys/random.h>
#endif

static long long g_tok_ffi;
static int g_ffi_ready;

static void oo_ffi_init(void) {
  unsigned char b[8];
  size_t i;
  unsigned long long acc;
  if (g_ffi_ready) return;
#if defined(__linux__) || defined(__APPLE__)
  if (getentropy(b, sizeof b) != 0)
#endif
  {
    acc = (unsigned long long)(uintptr_t)&g_tok_ffi;
    acc ^= (unsigned long long)getpid() << 16;
    acc ^= (unsigned long long)oo_monotonic_us();
    for (i = 0; i < sizeof b; i++) {
      acc = acc * 0x9E3779B97F4A7C15ULL + (unsigned long long)i;
      b[i] = (unsigned char)(acc >> 8);
    }
  }
  g_tok_ffi = 0x500000000LL | (long long)((b[0] << 24) | (b[1] << 16) | (b[2] << 8) | b[3]);
  if (g_tok_ffi == 0x4F4F4649LL) g_tok_ffi ^= 0x11111111LL;
  g_ffi_ready = 1;
}

long long oo_cap_grant_ffi(void) {
  oo_ffi_init();
  return g_tok_ffi;
}

void oo_cap_require_ffi(long long got, const char *op) {
  oo_ffi_init();
  if (got != g_tok_ffi) {
    fprintf(stderr, "ERR\tcap\t%s: missing or forged capability\n",
            op ? op : "ffi");
    exit(1);
  }
}

/* Path A OS dlopen: only when OODA_FFI_ALLOW_DLOPEN=1 and path is under
 * OODA_FFI_ALLOWDIR (absolute prefix). Otherwise residual Err after seal. */
static int path_under_allowdir(const char *path, const char *dir) {
  size_t n;
  if (!path || !dir || path[0] != '/' || dir[0] != '/') return 0;
  n = strlen(dir);
  if (n == 0) return 0;
  if (strncmp(path, dir, n) != 0) return 0;
  if (path[n] != '\0' && path[n] != '/') return 0;
  return 1;
}

OoResS oo_dlopen(long long cap, OoStr path) {
  OoResS r;
  const char *allow;
  const char *dir;
  const char *p;
  void *h;
  char buf[96];
  oo_cap_require_ffi(cap, "dlopen");
  r.ok = 0;
  allow = getenv("OODA_FFI_ALLOW_DLOPEN");
  dir = getenv("OODA_FFI_ALLOWDIR");
  p = path.data ? path.data : "";
  if (!allow || strcmp(allow, "1") != 0 || !dir || !dir[0]) {
    r.val = oo_str_lit("ffi residual: set OODA_FFI_ALLOW_DLOPEN=1 and OODA_FFI_ALLOWDIR for OS dlopen");
    return r;
  }
  if (!path_under_allowdir(p, dir)) {
    r.val = oo_str_lit("ffi residual: path not under OODA_FFI_ALLOWDIR");
    return r;
  }
  h = dlopen(p, RTLD_NOW);
  if (!h) {
    r.val = oo_str_lit("dlopen failed");
    return r;
  }
  /* Return opaque handle as hex string (path A; no dlsym product surface). */
  snprintf(buf, sizeof buf, "handle:%p", h);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}
