/* Compact SHA-256. Not chs_rt*.c. Used by oo_holo Merkle. */
#ifndef OO_SHA256_H
#define OO_SHA256_H

#include <stddef.h>

void oo_sha256(const unsigned char *data, size_t n, unsigned char out[32]);

#endif
