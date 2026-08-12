#include "chs_rt.h"
#include <limits.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <dirent.h>

/* Cap seals: process-local tokens from chs_rt_sys.c (R1 / CAP-G2).
 * require_fsread/fswrite accept granular OR full FsCap (g_tok_fs). */
void oo_cap_require_fsread(long long got, const char *op);
void oo_cap_require_fswrite(long long got, const char *op);
void oo_cap_require_env(long long got, const char *op);

/* Split absolute path into parent dir + final basename.
 * parent_out gets the parent path (not necessarily realpath'd).
 * Returns basename pointer into `path`, or NULL on refuse. */
static const char *fs_split_parent(const char *path, char *parent_out, size_t parent_sz) {
  const char *slash;
  size_t plen;
  if (!path || path[0] != '/' || !parent_out || parent_sz < 2) return NULL;
  slash = strrchr(path, '/');
  if (!slash) return NULL;
  if (slash == path) {
    /* "/leaf" → parent "/" */
    if (path[1] == '\0') return NULL; /* path is "/" alone */
    parent_out[0] = '/';
    parent_out[1] = '\0';
    return path + 1;
  }
  plen = (size_t)(slash - path);
  if (plen + 1 > parent_sz) return NULL;
  memcpy(parent_out, path, plen);
  parent_out[plen] = '\0';
  if (slash[1] == '\0') return NULL; /* trailing slash */
  return slash + 1;
}

/* Canonical-path prefix allow (mirror chs_rt_ffi.c path_under_allowdir).
 * Closes `..` traversal, symlink hops, and case-insensitive FS games.
 * Refuses root "/" as the allowed dir.
 * Create case: if leaf does not exist, realpath(parent) + basename check
 * (basename must not be "."/".."/empty). Residual: TOCTOU between check
 * and open is reduced by openat(O_NOFOLLOW) on the resolved parent. */
static int path_under_writedir(const char *path, const char *dir) {
  char rp_path[PATH_MAX];
  char rp_dir[PATH_MAX];
  char parent[PATH_MAX];
  const char *base;
  size_t n;
  if (!path || !dir || path[0] != '/' || dir[0] != '/') return 0;
  if (strcmp(dir, "/") == 0) return 0;
  if (!realpath(dir, rp_dir)) return 0;
  n = strlen(rp_dir);
  if (n == 0) return 0;

  /* Existing path: full realpath prefix compare. */
  if (realpath(path, rp_path)) {
    if (strncmp(rp_path, rp_dir, n) != 0) return 0;
    if (rp_path[n] != '\0' && rp_path[n] != '/') return 0;
    return 1;
  }

  /* Create / missing leaf: resolve parent only; require parent under dir. */
  base = fs_split_parent(path, parent, sizeof parent);
  if (!base || !base[0]) return 0;
  if (strcmp(base, ".") == 0 || strcmp(base, "..") == 0) return 0;
  if (strchr(base, '/')) return 0;
  if (!realpath(parent, rp_path)) return 0;
  if (strncmp(rp_path, rp_dir, n) != 0) return 0;
  if (rp_path[n] != '\0' && rp_path[n] != '/') return 0;
  return 1;
}

/* Open path for write truncate under resolved parent with O_NOFOLLOW leaf.
 * Reduces symlink-escape after allow: intermediate hops resolved via
 * realpath(parent); leaf symlink → ELOOP (fail-closed). Residual: rename
 * races on the parent dentry between realpath and openat (no landlock). */
static int writedir_open_trunc(const char *path) {
  char parent[PATH_MAX];
  char rp_parent[PATH_MAX];
  const char *base;
  int dfd, fd;
  base = fs_split_parent(path, parent, sizeof parent);
  if (!base || !base[0]) return -1;
  if (strcmp(base, ".") == 0 || strcmp(base, "..") == 0) return -1;
  if (!realpath(parent, rp_parent)) return -1;
  dfd = open(rp_parent, O_RDONLY | O_DIRECTORY | O_CLOEXEC);
  if (dfd < 0) return -1;
  fd = openat(dfd, base,
              O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC | O_NOFOLLOW, 0666);
  close(dfd);
  return fd;
}

OoResS oo_read_file(long long cap, OoStr path) {
  oo_cap_require_fsread(cap, "read_file");
  OoResS r;
  FILE *f = fopen(path.data, "rb");
  if (!f) {
    r.ok = 0;
    r.val = oo_str_lit("read_file failed");
    return r;
  }
  if (fseek(f, 0, SEEK_END) != 0) {
    fclose(f);
    r.ok = 0;
    r.val = oo_str_lit("read_file failed");
    return r;
  }
  long sz = ftell(f);
  if (sz < 0) {
    fclose(f);
    r.ok = 0;
    r.val = oo_str_lit("read_file failed");
    return r;
  }
  if (fseek(f, 0, SEEK_SET) != 0) {
    fclose(f);
    r.ok = 0;
    r.val = oo_str_lit("read_file failed");
    return r;
  }
  char *buf = oo_str_alloc_payload((size_t)sz);
  size_t n = fread(buf, 1, (size_t)sz, f);
  if (ferror(f)) {
    OoStr tmp = {buf, (long long)n};
    oo_str_release(tmp);
    fclose(f);
    r.ok = 0;
    r.val = oo_str_lit("read_file failed");
    return r;
  }
  buf[n] = 0;
  fclose(f);
  r.ok = 1;
  r.val.data = buf;
  r.val.len = (long long)n;
  return r;
}

OoResV oo_write_file(long long cap, OoStr path, OoStr content) {
  oo_cap_require_fswrite(cap, "write_file");
  OoResV r;
  const char *p;
  const char *dir;
  int fd;
  FILE *f;
  /* HIGH 6.4: write allowlist — path must canonicalize under OODA_FS_WRITEDIR.
   * Empty/unset dir → fail closed (no any-path writes). Policy key via
   * oo_process_policy_getenv only (OODA_* prefix). */
  p = path.data ? path.data : "";
  dir = oo_process_policy_getenv("OODA_FS_WRITEDIR");
  if (!dir || !dir[0] || !path_under_writedir(p, dir)) {
    r.ok = 0;
    r.err = oo_str_lit("write_file denied: path not under OODA_FS_WRITEDIR");
    return r;
  }
  fd = writedir_open_trunc(p);
  if (fd < 0) {
    r.ok = 0;
    r.err = oo_str_lit("write_file failed");
    return r;
  }
  f = fdopen(fd, "wb");
  if (!f) {
    close(fd);
    r.ok = 0;
    r.err = oo_str_lit("write_file failed");
    return r;
  }
  /* Torn-state seal: short write / ferror / fclose fail → Err, never ok=1. */
  size_t want = content.data ? (size_t)content.len : 0;
  size_t nw = want ? fwrite(content.data, 1, want, f) : 0;
  int bad = (nw != want) || ferror(f);
  if (fclose(f) != 0) {
    bad = 1;
  }
  if (bad) {
    r.ok = 0;
    r.err = oo_str_lit("write_file failed");
    return r;
  }
  r.ok = 1;
  r.err = oo_str_lit("");
  return r;
}

int oo_path_exists(long long cap, OoStr path) {
  oo_cap_require_fsread(cap, "path_exists");
  FILE *f = fopen(path.data, "rb");
  if (f) {
    fclose(f);
    return 1;
  }
  return 0;
}

long long oo_file_size(long long cap, OoStr path) {
  oo_cap_require_fsread(cap, "file_size");
  FILE *f = fopen(path.data, "rb");
  if (!f) return -1;
  fseek(f, 0, SEEK_END);
  long long sz = ftell(f);
  fclose(f);
  return sz;
}

OoResS oo_env_get(long long cap, OoStr key) {
  oo_cap_require_env(cap, "env_get");
  OoResS r;
  const char *val = oo_process_policy_getenv(key.data ? key.data : "");
  if (val) {
    r.ok = 1;
    r.val = oo_str_lit(val);
  } else {
    r.ok = 0;
    r.val = oo_str_lit("env var not set");
  }
  return r;
}

long long oo_monotonic_us(void) {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  long long us = (long long)ts.tv_sec * 1000000LL + (long long)ts.tv_nsec / 1000LL;
  return us > 0LL ? us : 1LL;
}

OoSList fs_read_dir(long long cap, OoStr path) {
  oo_cap_require_fsread(cap, "fs_read_dir");
  OoSList l = oo_slist_new();
  const char *p = path.data ? path.data : "";
  DIR *d = opendir(p);
  if (!d) return l;
  struct dirent *dir;
  while ((dir = readdir(d)) != NULL) {
    if (strcmp(dir->d_name, ".") == 0 || strcmp(dir->d_name, "..") == 0) continue;
    OoStr part = oo_str_lit(dir->d_name);
    OoSList next = oo_slist_push(l, part);
    oo_slist_release(l);
    l = next;
    oo_str_release(part);
  }
  closedir(d);
  return l;
}

int fs_is_dir(long long cap, OoStr path) {
  oo_cap_require_fsread(cap, "fs_is_dir");
  char cpath[1024];
  long long n = path.len;
  if (n >= 1024) n = 1023;
  memcpy(cpath, path.data ? path.data : "", n);
  cpath[n] = '\0';
  struct stat st;
  if (stat(cpath, &st) == 0) {
    return S_ISDIR(st.st_mode) ? 1 : 0;
  }
  return 0;
}
