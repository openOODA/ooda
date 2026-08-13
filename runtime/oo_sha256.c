/* FIPS 180-4 SHA-256. New file. Not a crypto product claim beyond the digest. */
#include "oo_sha256.h"
#include <string.h>
#include <stdint.h>
#include <stdlib.h>

#define RR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
#define CH(x, y, z) (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define S0(x) (RR((x), 2) ^ RR((x), 13) ^ RR((x), 22))
#define S1(x) (RR((x), 6) ^ RR((x), 11) ^ RR((x), 25))
#define s0(x) (RR((x), 7) ^ RR((x), 18) ^ ((x) >> 3))
#define s1(x) (RR((x), 17) ^ RR((x), 19) ^ ((x) >> 10))

static const uint32_t K[64] = {
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

void oo_sha256(const unsigned char *data, size_t n, unsigned char out[32]) {
  uint32_t s[8] = {
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
  };
  uint64_t bits = (uint64_t)n * 8;
  size_t pad = n + 1;
  size_t total;
  unsigned char *buf;
  size_t off;
  int i;
  if (!out) {
    return;
  }
  while ((pad % 64) != 56) {
    pad += 1;
  }
  total = pad + 8;
  buf = (unsigned char *)calloc(total, 1);
  if (!buf) {
    memset(out, 0, 32);
    return;
  }
  if (n > 0 && data) {
    memcpy(buf, data, n);
  }
  buf[n] = 0x80;
  for (i = 0; i < 8; i++) {
    buf[total - 1 - i] = (unsigned char)(bits >> (i * 8));
  }
  for (off = 0; off < total; off += 64) {
    uint32_t W[64];
    uint32_t a, b, c, d, e, f, g, h;
    const unsigned char *p = buf + off;
    for (i = 0; i < 16; i++) {
      W[i] = ((uint32_t)p[i * 4] << 24) | ((uint32_t)p[i * 4 + 1] << 16) |
             ((uint32_t)p[i * 4 + 2] << 8) | ((uint32_t)p[i * 4 + 3]);
    }
    for (i = 16; i < 64; i++) {
      W[i] = W[i - 16] + s0(W[i - 15]) + W[i - 7] + s1(W[i - 2]);
    }
    a = s[0]; b = s[1]; c = s[2]; d = s[3];
    e = s[4]; f = s[5]; g = s[6]; h = s[7];
    for (i = 0; i < 64; i++) {
      uint32_t t1 = h + S1(e) + CH(e, f, g) + K[i] + W[i];
      uint32_t t2 = S0(a) + MAJ(a, b, c);
      h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    s[0] += a; s[1] += b; s[2] += c; s[3] += d;
    s[4] += e; s[5] += f; s[6] += g; s[7] += h;
  }
  free(buf);
  for (i = 0; i < 8; i++) {
    out[i * 4] = (unsigned char)(s[i] >> 24);
    out[i * 4 + 1] = (unsigned char)(s[i] >> 16);
    out[i * 4 + 2] = (unsigned char)(s[i] >> 8);
    out[i * 4 + 3] = (unsigned char)(s[i]);
  }
}
