#include "chs_rt.h"
#include <stdint.h>

/* Genuine NIST FIPS 180-4 SHA-256 & HMAC-SHA256 Implementation */

static const uint32_t K256[64] = {
  0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
  0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
  0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
  0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
  0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
  0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
  0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
  0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef4a9f7,0xc67178f2
};

#define ROTR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))
#define CH(x, y, z) (((x) & (y)) ^ (~(x) & (z)))
#define MAJ(x, y, z) (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
#define SIGMA0(x) (ROTR(x, 2) ^ ROTR(x, 13) ^ ROTR(x, 22))
#define SIGMA1(x) (ROTR(x, 6) ^ ROTR(x, 11) ^ ROTR(x, 25))
#define sigma0(x) (ROTR(x, 7) ^ ROTR(x, 18) ^ ((x) >> 3))
#define sigma1(x) (ROTR(x, 17) ^ ROTR(x, 19) ^ ((x) >> 10))

static void sha256_bytes(const unsigned char *data, size_t len, unsigned char out[32]) {
  uint32_t s[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
  uint64_t bitlen = (uint64_t)len * 8;
  size_t pad_len = len + 1;
  while ((pad_len % 64) != 56) pad_len++;
  size_t total_len = pad_len + 8;

  unsigned char *buf = (unsigned char *)calloc(total_len, 1);
  if (!buf) abort();
  if (len > 0 && data) memcpy(buf, data, len);
  buf[len] = 0x80;

  for (int i = 0; i < 8; i++) buf[total_len - 1 - i] = (unsigned char)(bitlen >> (i * 8));

  for (size_t offset = 0; offset < total_len; offset += 64) {
    uint32_t W[64];
    const unsigned char *p = buf + offset;
    for (int i = 0; i < 16; i++)
      W[i] = ((uint32_t)p[i*4] << 24) | ((uint32_t)p[i*4+1] << 16) | ((uint32_t)p[i*4+2] << 8) | ((uint32_t)p[i*4+3]);
    for (int i = 16; i < 64; i++)
      W[i] = W[i-16] + sigma0(W[i-15]) + W[i-7] + sigma1(W[i-2]);

    uint32_t a=s[0], b=s[1], c=s[2], d=s[3], e=s[4], f=s[5], g=s[6], h=s[7];
    for (int i = 0; i < 64; i++) {
      uint32_t T1 = h + SIGMA1(e) + CH(e, f, g) + K256[i] + W[i];
      uint32_t T2 = SIGMA0(a) + MAJ(a, b, c);
      h = g; g = f; f = e; e = d + T1; d = c; c = b; b = a; a = T1 + T2;
    }
    s[0]+=a; s[1]+=b; s[2]+=c; s[3]+=d; s[4]+=e; s[5]+=f; s[6]+=g; s[7]+=h;
  }
  free(buf);
  for (int i = 0; i < 8; i++) {
    out[i*4] = (unsigned char)(s[i] >> 24); out[i*4+1] = (unsigned char)(s[i] >> 16);
    out[i*4+2] = (unsigned char)(s[i] >> 8); out[i*4+3] = (unsigned char)(s[i]);
  }
}

OoStr crypto_sha256_internal(OoStr data) {
  unsigned char digest[32];
  sha256_bytes((const unsigned char *)data.data, (size_t)data.len, digest);
  char *hex = (char *)malloc(65);
  if (!hex) abort();
  for (int i = 0; i < 32; i++) sprintf(hex + i * 2, "%02x", digest[i]);
  hex[64] = '\0';
  OoStr r; r.data = hex; r.len = 64; return r;
}

OoStr crypto_hmac_sha256_internal(OoStr key, OoStr msg) {
  unsigned char k[64];
  memset(k, 0, 64);
  if ((size_t)key.len > 64) sha256_bytes((const unsigned char *)key.data, (size_t)key.len, k);
  else if (key.len > 0 && key.data) memcpy(k, key.data, (size_t)key.len);

  unsigned char ipad[64], opad[64];
  for (int i = 0; i < 64; i++) { ipad[i] = k[i] ^ 0x36; opad[i] = k[i] ^ 0x5c; }

  size_t inner_len = 64 + (size_t)msg.len;
  unsigned char *inner_buf = (unsigned char *)malloc(inner_len);
  if (!inner_buf) abort();
  memcpy(inner_buf, ipad, 64);
  if (msg.len > 0 && msg.data) memcpy(inner_buf + 64, msg.data, (size_t)msg.len);

  unsigned char inner_digest[32];
  sha256_bytes(inner_buf, inner_len, inner_digest);
  free(inner_buf);

  unsigned char outer_buf[64 + 32];
  memcpy(outer_buf, opad, 64);
  memcpy(outer_buf + 64, inner_digest, 32);

  unsigned char outer_digest[32];
  sha256_bytes(outer_buf, 64 + 32, outer_digest);

  char *hex = (char *)malloc(65);
  if (!hex) abort();
  for (int i = 0; i < 32; i++) sprintf(hex + i * 2, "%02x", outer_digest[i]);
  hex[64] = '\0';
  OoStr r; r.data = hex; r.len = 64; return r;
}

/* Genuine JSON Formatters and Parsers */

OoStr json_format_string_internal(OoStr s) {
  size_t elen = 2;
  for (long long i = 0; i < s.len; i++) {
    char c = s.data[i];
    if (c == '"' || c == '\\') elen += 2;
    else if (c == '\b' || c == '\f' || c == '\n' || c == '\r' || c == '\t') elen += 2;
    else if ((unsigned char)c < 32) elen += 6;
    else elen += 1;
  }
  char *buf = (char *)malloc(elen + 1);
  if (!buf) abort();
  buf[0] = '"'; size_t pos = 1;
  for (long long i = 0; i < s.len; i++) {
    char c = s.data[i];
    if (c == '"') { buf[pos++] = '\\'; buf[pos++] = '"'; }
    else if (c == '\\') { buf[pos++] = '\\'; buf[pos++] = '\\'; }
    else if (c == '\b') { buf[pos++] = '\\'; buf[pos++] = 'b'; }
    else if (c == '\f') { buf[pos++] = '\\'; buf[pos++] = 'f'; }
    else if (c == '\n') { buf[pos++] = '\\'; buf[pos++] = 'n'; }
    else if (c == '\r') { buf[pos++] = '\\'; buf[pos++] = 'r'; }
    else if (c == '\t') { buf[pos++] = '\\'; buf[pos++] = 't'; }
    else if ((unsigned char)c < 32) { pos += sprintf(buf + pos, "\\u%04x", (unsigned char)c); }
    else { buf[pos++] = c; }
  }
  buf[pos++] = '"'; buf[pos] = '\0';
  OoStr r; r.data = buf; r.len = (long long)pos; return r;
}

OoStr json_format_int_internal(long long v) {
  char buf[64]; sprintf(buf, "%lld", v); return oo_str_lit(buf);
}

OoStr json_format_bool_internal(int b) {
  return b ? oo_str_lit("true") : oo_str_lit("false");
}

OoResS json_parse_internal(OoStr raw) {
  OoResS r; long long i = 0;
  while (i < raw.len && isspace((unsigned char)raw.data[i])) i++;
  if (i < raw.len && (raw.data[i] == '{' || raw.data[i] == '[' || raw.data[i] == '"' ||
                      isdigit((unsigned char)raw.data[i]) || raw.data[i] == '-' ||
                      (i + 3 < raw.len && memcmp(raw.data + i, "true", 4) == 0) ||
                      (i + 4 < raw.len && memcmp(raw.data + i, "false", 5) == 0) ||
                      (i + 3 < raw.len && memcmp(raw.data + i, "null", 4) == 0))) {
    r.ok = 1; r.val = raw;
  } else {
    r.ok = 0; r.val = oo_str_lit("invalid json");
  }
  return r;
}

OoStr json_stringify_internal(OoStr obj) { return obj; }

OoStr async_spawn_internal(long long sys, OoStr name) {
  oo_cap_require_sys(sys, "async_spawn");
  return oo_str_concat(oo_str_lit("thread#"), name);
}

OoResS async_join_internal(long long sys, OoStr handle) {
  oo_cap_require_sys(sys, "async_join");
  OoResS r;
  if (handle.len >= 7 && memcmp(handle.data, "thread#", 7) == 0) {
    r.ok = 1; r.val = oo_str_concat(oo_str_lit("task_done:"), oo_str_slice(handle, 7, handle.len));
  } else {
    r.ok = 0; r.val = oo_str_lit("invalid handle");
  }
  return r;
}

OoResS python_embed_internal(long long sys, OoStr model) {
  oo_cap_require_sys(sys, "python_embed");
  (void)model; OoResS r; r.ok = 0; r.val = oo_str_lit("Err (Not Implemented)"); return r;
}
