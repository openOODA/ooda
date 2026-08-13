/* Merkle persist of a byte blob. Not language holo_persist (still refused). */
#ifndef OO_HOLO_H
#define OO_HOLO_H

#include <stddef.h>

#define OO_HOLO_ROOT 32
#define OO_HOLO_LEAF 64

int oo_holo_root(const unsigned char *data, size_t n, unsigned char root[32]);
int oo_holo_path_ok(const char *path);
int oo_holo_persist(const char *path, const unsigned char *data, size_t n);
int oo_holo_load(const char *path, unsigned char *out, size_t cap, size_t *n_out,
                 unsigned char root[32]);

#endif
