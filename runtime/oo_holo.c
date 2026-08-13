/* Fail-closed Merkle blob store. File I/O is explicit-path only. No net. */
#include "oo_holo.h"
#include "oo_sha256.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

static int write_u64le(FILE *f, uint64_t v) {
  unsigned char b[8];
  int i;
  for (i = 0; i < 8; i++) {
    b[i] = (unsigned char)(v & 0xff);
    v >>= 8;
  }
  return fwrite(b, 1, 8, f) == 8 ? 0 : -1;
}

static int read_u64le(FILE *f, uint64_t *out) {
  unsigned char b[8];
  uint64_t v = 0;
  int i;
  if (fread(b, 1, 8, f) != 8) {
    return -1;
  }
  for (i = 7; i >= 0; i--) {
    v = (v << 8) | (uint64_t)b[i];
  }
  *out = v;
  return 0;
}

int oo_holo_path_ok(const char *path) {
  size_t i;
  if (!path || path[0] == 0) {
    return 0;
  }
  for (i = 0; path[i]; i++) {
    if (path[i] == '.' && path[i + 1] == '.') {
      return 0;
    }
  }
  return 1;
}

int oo_holo_root(const unsigned char *data, size_t n, unsigned char root[32]) {
  size_t nleaves;
  size_t i;
  unsigned char *level;
  unsigned char leaf[OO_HOLO_LEAF];
  if (!root) {
    return -1;
  }
  if (n > 0 && !data) {
    return -1;
  }
  nleaves = (n + OO_HOLO_LEAF - 1) / OO_HOLO_LEAF;
  if (nleaves == 0) {
    nleaves = 1;
  }
  level = (unsigned char *)calloc(nleaves, 32);
  if (!level) {
    return -1;
  }
  for (i = 0; i < nleaves; i++) {
    size_t off = i * OO_HOLO_LEAF;
    size_t take = 0;
    memset(leaf, 0, sizeof leaf);
    if (off < n) {
      take = n - off;
      if (take > OO_HOLO_LEAF) {
        take = OO_HOLO_LEAF;
      }
      memcpy(leaf, data + off, take);
    }
    oo_sha256(leaf, OO_HOLO_LEAF, level + i * 32);
  }
  while (nleaves > 1) {
    size_t next = (nleaves + 1) / 2;
    unsigned char *up = (unsigned char *)calloc(next, 32);
    unsigned char pair[64];
    if (!up) {
      free(level);
      return -1;
    }
    for (i = 0; i < nleaves; i += 2) {
      memcpy(pair, level + i * 32, 32);
      if (i + 1 < nleaves) {
        memcpy(pair + 32, level + (i + 1) * 32, 32);
      } else {
        memcpy(pair + 32, level + i * 32, 32);
      }
      oo_sha256(pair, 64, up + (i / 2) * 32);
    }
    free(level);
    level = up;
    nleaves = next;
  }
  memcpy(root, level, 32);
  free(level);
  return 0;
}

int oo_holo_persist(const char *path, const unsigned char *data, size_t n) {
  FILE *f;
  unsigned char root[32];
  if (!oo_holo_path_ok(path)) {
    return -1;
  }
  if (n > 0 && !data) {
    return -1;
  }
  if (oo_holo_root(data, n, root) != 0) {
    return -1;
  }
  f = fopen(path, "wb");
  if (!f) {
    return -1;
  }
  if (fwrite("OOH1", 1, 4, f) != 4) {
    fclose(f);
    return -1;
  }
  if (write_u64le(f, (uint64_t)n) != 0) {
    fclose(f);
    return -1;
  }
  if (n > 0 && fwrite(data, 1, n, f) != n) {
    fclose(f);
    return -1;
  }
  if (fwrite(root, 1, 32, f) != 32) {
    fclose(f);
    return -1;
  }
  if (fclose(f) != 0) {
    return -1;
  }
  return 0;
}

int oo_holo_load(const char *path, unsigned char *out, size_t cap, size_t *n_out,
                 unsigned char root[32]) {
  FILE *f;
  char mag[4];
  uint64_t n64;
  size_t n;
  unsigned char stored[32];
  unsigned char got[32];
  if (!oo_holo_path_ok(path) || !out || !n_out || !root) {
    return -1;
  }
  f = fopen(path, "rb");
  if (!f) {
    return -1;
  }
  if (fread(mag, 1, 4, f) != 4 || memcmp(mag, "OOH1", 4) != 0) {
    fclose(f);
    return -1;
  }
  if (read_u64le(f, &n64) != 0 || n64 > cap) {
    fclose(f);
    return -1;
  }
  n = (size_t)n64;
  if (n > 0 && fread(out, 1, n, f) != n) {
    fclose(f);
    return -1;
  }
  if (fread(stored, 1, 32, f) != 32) {
    fclose(f);
    return -1;
  }
  fclose(f);
  if (oo_holo_root(out, n, got) != 0) {
    return -1;
  }
  if (memcmp(stored, got, 32) != 0) {
    return -1;
  }
  memcpy(root, got, 32);
  *n_out = n;
  return 0;
}
