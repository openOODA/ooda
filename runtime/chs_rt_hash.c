/* M161: MD5 + SHA-1 pure digests (hex). AES residual stays fail-closed string elsewhere. */
#include "chs_rt.h"

#define LR(x, c) (((x) << (c)) | ((x) >> (32 - (c))))

static OoStr hex_encode_n(const unsigned char *d, size_t n) {
  static const char *hx = "0123456789abcdef";
  char *buf = oo_str_alloc_payload(n * 2);
  size_t i;
  for (i = 0; i < n; i++) {
    buf[i * 2] = hx[(d[i] >> 4) & 0xf];
    buf[i * 2 + 1] = hx[d[i] & 0xf];
  }
  OoStr r;
  r.data = buf;
  r.len = (long long)(n * 2);
  return r;
}

static void md5_bytes(const unsigned char *initial_msg, size_t initial_len, unsigned char *digest) {
  uint32_t h0 = 0x67452301, h1 = 0xefcdab89, h2 = 0x98badcfe, h3 = 0x10325476;
  size_t new_len, offset;
  uint64_t bits_len;
  unsigned char *msg = NULL;
  static const uint32_t r[] = {
    7,12,17,22,7,12,17,22,7,12,17,22,7,12,17,22,5,9,14,20,5,9,14,20,5,9,14,20,5,9,14,20,
    4,11,16,23,4,11,16,23,4,11,16,23,4,11,16,23,6,10,15,21,6,10,15,21,6,10,15,21,6,10,15,21};
  static const uint32_t k[] = {
    0xd76aa478,0xe8c7b756,0x242070db,0xc1bdceee,0xf57c0faf,0x4787c62a,0xa8304613,0xfd469501,
    0x698098d8,0x8b44f7af,0xffff5bb1,0x895cd7be,0x6b901122,0xfd987193,0xa679438e,0x49b40821,
    0xf61e2562,0xc040b340,0x265e5a51,0xe9b6c7aa,0xd62f105d,0x02441453,0xd8a1e681,0xe7d3fbc8,
    0x21e1cde6,0xc33707d6,0xf4d50d87,0x455a14ed,0xa9e3e905,0xfcefa3f8,0x676f02d9,0x8d2a4c8a,
    0xfffa3942,0x8771f681,0x6d9d6122,0xfde5380c,0xa4beea44,0x4bdecfa9,0xf6bb4b60,0xbebfbc70,
    0x289b7ec6,0xeaa127fa,0xd4ef3085,0x04881d05,0xd9d4d039,0xe6db99e5,0x1fa27cf8,0xc4ac5665,
    0xf4292244,0x432aff97,0xab9423a7,0xfc93a039,0x655b59c3,0x8f0ccc92,0xffeff47d,0x85845dd1,
    0x6fa87e4f,0xfe2ce6e0,0xa3014314,0x4e0811a1,0xf7537e82,0xbd3af235,0x2ad7d2bb,0xeb86d391};
  new_len = ((((initial_len + 8) / 64) + 1) * 64) - 8;
  msg = (unsigned char *)calloc(new_len + 64, 1);
  if (!msg) abort();
  if (initial_len && initial_msg) memcpy(msg, initial_msg, initial_len);
  msg[initial_len] = 128;
  bits_len = (uint64_t)initial_len * 8;
  memcpy(msg + new_len, &bits_len, 8);
  for (offset = 0; offset < new_len; offset += 64) {
    uint32_t *w = (uint32_t *)(msg + offset);
    uint32_t a = h0, b = h1, c = h2, d = h3, i, f, g, temp;
    for (i = 0; i < 64; i++) {
      if (i < 16) { f = (b & c) | ((~b) & d); g = i; }
      else if (i < 32) { f = (d & b) | ((~d) & c); g = (5 * i + 1) % 16; }
      else if (i < 48) { f = b ^ c ^ d; g = (3 * i + 5) % 16; }
      else { f = c ^ (b | (~d)); g = (7 * i) % 16; }
      temp = d; d = c; c = b;
      b = b + LR((a + f + k[i] + w[g]), r[i]);
      a = temp;
    }
    h0 += a; h1 += b; h2 += c; h3 += d;
  }
  free(msg);
  memcpy(digest, &h0, 4); memcpy(digest + 4, &h1, 4);
  memcpy(digest + 8, &h2, 4); memcpy(digest + 12, &h3, 4);
}

OoStr crypto_md5_internal(OoStr data) {
  unsigned char dig[16];
  const unsigned char *p = (const unsigned char *)(data.data ? data.data : "");
  size_t n = data.data ? (size_t)data.len : 0;
  md5_bytes(p, n, dig);
  return hex_encode_n(dig, 16);
}

static void sha1_bytes(const unsigned char *str, size_t len, unsigned char *hash) {
  uint32_t h0=0x67452301,h1=0xEFCDAB89,h2=0x98BADCFE,h3=0x10325476,h4=0xC3D2E1F0;
  uint64_t ml = (uint64_t)len * 8;
  size_t pad = (len % 64 < 56) ? (56 - len % 64) : (120 - len % 64);
  size_t total = len + pad + 8;
  unsigned char *msg = (unsigned char *)calloc(total, 1);
  size_t i, chunk;
  if (!msg) abort();
  if (len && str) memcpy(msg, str, len);
  msg[len] = 0x80;
  for (i = 0; i < 8; i++) msg[total - 1 - i] = (unsigned char)((ml >> (8 * i)) & 0xff);
  for (chunk = 0; chunk < total; chunk += 64) {
    uint32_t w[80], a,b,c,d,e,f,k,temp,j;
    for (i = 0; i < 16; i++) {
      w[i] = ((uint32_t)msg[chunk+i*4]<<24)|((uint32_t)msg[chunk+i*4+1]<<16)|
             ((uint32_t)msg[chunk+i*4+2]<<8)|(uint32_t)msg[chunk+i*4+3];
    }
    for (i = 16; i < 80; i++) {
      temp = w[i-3]^w[i-8]^w[i-14]^w[i-16];
      w[i] = LR(temp, 1);
    }
    a=h0;b=h1;c=h2;d=h3;e=h4;
    for (i = 0; i < 80; i++) {
      if (i < 20) { f = (b & c) | ((~b) & d); k = 0x5A827999; }
      else if (i < 40) { f = b ^ c ^ d; k = 0x6ED9EBA1; }
      else if (i < 60) { f = (b & c) | (b & d) | (c & d); k = 0x8F1BBCDC; }
      else { f = b ^ c ^ d; k = 0xCA62C1D6; }
      temp = LR(a, 5) + f + e + k + w[i];
      e = d; d = c; c = LR(b, 30); b = a; a = temp;
    }
    h0 += a; h1 += b; h2 += c; h3 += d; h4 += e;
  }
  free(msg);
  for (i = 0; i < 4; i++) {
    hash[i] = (h0 >> (24 - i * 8)) & 0xFF;
    hash[4 + i] = (h1 >> (24 - i * 8)) & 0xFF;
    hash[8 + i] = (h2 >> (24 - i * 8)) & 0xFF;
    hash[12 + i] = (h3 >> (24 - i * 8)) & 0xFF;
    hash[16 + i] = (h4 >> (24 - i * 8)) & 0xFF;
  }
}

OoStr crypto_sha1_internal(OoStr data) {
  unsigned char dig[20];
  const unsigned char *p = (const unsigned char *)(data.data ? data.data : "");
  size_t n = data.data ? (size_t)data.len : 0;
  sha1_bytes(p, n, dig);
  return hex_encode_n(dig, 20);
}

OoStr crypto_aes_encrypt_internal(OoStr key, OoStr plain) {
  (void)key;
  (void)plain;
  return oo_str_lit("STUB_FAIL_CLOSED");
}
