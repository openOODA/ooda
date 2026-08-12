/* M156/M162/M165: process-local UnsafeFFICap + allowlisted OS dlopen Path A.
 * Path A also: registered-handle dlsym/dlclose (no typed ffi_call of symbols). */
#include "chs_rt.h"
#include <stdlib.h>
#include <limits.h>
#include <fcntl.h>
#include <unistd.h>
#include <dlfcn.h>
#include <string.h>
#include <pthread.h>
#if defined(__linux__) || defined(__APPLE__)
#include <sys/random.h>
#endif

static pthread_once_t g_ffi_once = PTHREAD_ONCE_INIT;
static long long g_tok_ffi;

static void ffi_once_init(void) {
  unsigned char b[8];
  size_t i;
  unsigned long long acc;
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
  g_tok_ffi = 0x5000000000000000LL | (long long)((((unsigned long long)b[0]) << 56) | (((unsigned long long)b[1]) << 48) | (((unsigned long long)b[2]) << 40) | (((unsigned long long)b[3]) << 32) | (((unsigned long long)b[4]) << 24) | (((unsigned long long)b[5]) << 16) | (((unsigned long long)b[6]) << 8) | ((unsigned long long)b[7]));
  if (g_tok_ffi == 0x4F4F4649LL) g_tok_ffi ^= 0x11111111LL;
}

static void oo_ffi_init(void) {
  pthread_once(&g_ffi_once, ffi_once_init);
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

/* Canonical-path prefix allow: both inputs run through realpath and the
 * canonical paths are compared. Closes `..` traversal, symlink hops,
 * and case-insensitive FS games. */
static int path_under_allowdir(const char *path, const char *dir) {
  char rp_path[PATH_MAX];
  char rp_dir[PATH_MAX];
  size_t n;
  if (!path || !dir || path[0] != '/' || dir[0] != '/') return 0;
  /* Reject root — operators who set "/" mean to allow everything; we refuse. */
  if (strcmp(dir, "/") == 0) return 0;
  if (!realpath(path, rp_path)) return 0;
  if (!realpath(dir, rp_dir)) return 0;
  n = strlen(rp_dir);
  if (n == 0 || strncmp(rp_path, rp_dir, n) != 0) return 0;
  if (rp_path[n] != '\0' && rp_path[n] != '/') return 0;
  return 1;
}

/* M165: safe system lib dirs when ALLOWDIR empty (not unrestricted any-path). */
static int path_under_sys_lib(const char *path) {
  return path_under_allowdir(path, "/lib")
      || path_under_allowdir(path, "/lib64")
      || path_under_allowdir(path, "/usr/lib")
      || path_under_allowdir(path, "/usr/lib64");
}

/* Path A OS dlopen:
 * - Without OODA_FFI_ALLOW_DLOPEN=1 → residual Err after seal
 * - ALLOW=1 + ALLOWDIR set → prefix allowlist (M162)
 * - ALLOW=1 + ALLOWDIR empty/unset → only /lib|/lib64|/usr/lib|/usr/lib64
 * TOCTOU: allow check + open + dlopen is best-effort reduced via re-realpath
 * (canonical path for open/dlopen) + O_NOFOLLOW at leaf. Full fdlopen is not
 * portable. Still residual: unrestricted any-path load; no ffi_call of
 * looked-up symbols as product. */
#define OO_FFI_HANDLE_SLOTS 16
static void *g_ffi_handles[OO_FFI_HANDLE_SLOTS];
static pthread_mutex_t g_ffi_handles_mu = PTHREAD_MUTEX_INITIALIZER;

/* Register h in a free slot; 0 on success, -1 if full. Idempotent if h already
 * present. Keys are the returned handle:%p strings (lookup by format match). */
static int ffi_handle_register(void *h) {
  int i;
  int free_i = -1;
  if (!h) return -1;
  pthread_mutex_lock(&g_ffi_handles_mu);
  for (i = 0; i < OO_FFI_HANDLE_SLOTS; i++) {
    if (g_ffi_handles[i] == h) { pthread_mutex_unlock(&g_ffi_handles_mu); return 0; }
    if (g_ffi_handles[i] == NULL && free_i < 0) free_i = i;
  }
  if (free_i < 0) { pthread_mutex_unlock(&g_ffi_handles_mu); return -1; }
  g_ffi_handles[free_i] = h;
  pthread_mutex_unlock(&g_ffi_handles_mu);
  return 0;
}

/* Lookup by handle string key "handle:%p". Returns slot index or -1. */
static int ffi_handle_lookup(const char *key, void **out) {
  int i;
  int rc;
  char buf[96];
  if (!key || !key[0]) return -1;
  pthread_mutex_lock(&g_ffi_handles_mu);
  for (i = 0; i < OO_FFI_HANDLE_SLOTS; i++) {
    if (!g_ffi_handles[i]) continue;
    snprintf(buf, sizeof buf, "handle:%p", g_ffi_handles[i]);
    if (strcmp(buf, key) == 0) {
      if (out) *out = g_ffi_handles[i];
      rc = i;
      pthread_mutex_unlock(&g_ffi_handles_mu);
      return rc;
    }
  }
  pthread_mutex_unlock(&g_ffi_handles_mu);
  return -1;
}

static void ffi_handle_clear(int slot) {
  if (slot < 0 || slot >= OO_FFI_HANDLE_SLOTS) return;
  pthread_mutex_lock(&g_ffi_handles_mu);
  g_ffi_handles[slot] = NULL;
  pthread_mutex_unlock(&g_ffi_handles_mu);
}

OoResS oo_dlopen(long long cap, OoStr path) {
  OoResS r;
  const char *allow;
  const char *dir;
  const char *p;
  void *h;
  char buf[96];
  char rp_path[PATH_MAX];
  oo_cap_require_ffi(cap, "dlopen");
  r.ok = 0;
  /* ZT: process-policy keys only (OODA_*) — not product env_get */
  allow = oo_process_policy_getenv("OODA_FFI_ALLOW_DLOPEN");
  dir = oo_process_policy_getenv("OODA_FFI_ALLOWDIR");
  p = path.data ? path.data : "";
  if (!allow || strcmp(allow, "1") != 0) {
    r.val = oo_str_lit("ffi residual: set OODA_FFI_ALLOW_DLOPEN=1 for OS dlopen");
    return r;
  }
  if (dir && dir[0]) {
    if (!path_under_allowdir(p, dir)) {
      r.val = oo_str_lit("ffi residual: path not under OODA_FFI_ALLOWDIR");
      return r;
    }
  } else if (!path_under_sys_lib(p)) {
    r.val = oo_str_lit("ffi residual: path not under system lib dirs");
    return r;
  }
  /* Re-canonicalize after allow check; open/dlopen must use rp_path, not p. */
  if (!realpath(p, rp_path)) {
    r.val = oo_str_lit("ffi residual: realpath failed after allow check");
    return r;
  }
  /* Reject symlinks at the leaf on the canonical path (O_NOFOLLOW). */
  {
    int fd = open(rp_path, O_RDONLY | O_NOFOLLOW);
    if (fd < 0) {
      r.val = oo_str_lit("ffi residual: leaf is symlink or not openable");
      return r;
    }
    close(fd);
  }
  h = dlopen(rp_path, RTLD_NOW);
  if (!h) {
    r.val = oo_str_lit("dlopen failed");
    return r;
  }
  if (ffi_handle_register(h) != 0) {
    dlclose(h);
    r.val = oo_str_lit("ffi residual: handle table full");
    return r;
  }
  snprintf(buf, sizeof buf, "handle:%p", h);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}

/* Path A: real dlsym for handles registered by oo_dlopen.
 * Residual: no typed ffi_call through the returned symbol pointer. */
OoResS oo_dlsym(long long cap, OoStr handle, OoStr name) {
  OoResS r;
  const char *hk;
  const char *nm;
  void *h = NULL;
  void *sym;
  char buf[96];
  oo_cap_require_ffi(cap, "dlsym");
  r.ok = 0;
  hk = handle.data ? handle.data : "";
  nm = name.data ? name.data : "";
  if (ffi_handle_lookup(hk, &h) < 0 || !h) {
    r.val = oo_str_lit("ffi residual: unknown handle");
    return r;
  }
  if (!nm[0]) {
    r.val = oo_str_lit("ffi residual: empty symbol name");
    return r;
  }
  sym = dlsym(h, nm);
  if (!sym) {
    r.val = oo_str_lit("dlsym failed");
    return r;
  }
  /* Product: opaque symbol id only — no call convention / typed invoke yet. */
  snprintf(buf, sizeof buf, "sym:%p", sym);
  r.ok = 1;
  r.val = oo_str_lit(buf);
  return r;
}

/* Path A: dlclose + free table slot for registered handles only. */
OoResS oo_dlclose(long long cap, OoStr handle) {
  OoResS r;
  const char *hk;
  void *h = NULL;
  int slot;
  oo_cap_require_ffi(cap, "dlclose");
  r.ok = 0;
  hk = handle.data ? handle.data : "";
  slot = ffi_handle_lookup(hk, &h);
  if (slot < 0 || !h) {
    r.val = oo_str_lit("ffi residual: unknown handle");
    return r;
  }
  dlclose(h);
  ffi_handle_clear(slot);
  r.ok = 1;
  r.val = oo_str_lit("closed");
  return r;
}
