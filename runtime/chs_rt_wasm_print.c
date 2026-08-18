#include "chs_rt.h"
/* WASI fd_write body for last note_fn. If none yet, pending binds to next
   note_fn; if emit runs first, to main if named else fn 0. */
#define OO_WASM_PBUF 4096u
#define OO_WASM_DATA 16u
#define OO_WASM_PAY_MAX 24000u
#define OO_WASM_MOD_MAX 28000u
static unsigned char oo_wasm_pbuf[OO_WASM_PBUF];
static unsigned int oo_wasm_pused;
static unsigned int oo_wasm_poff[OO_WASM_FN_CAP];
static unsigned int oo_wasm_plen[OO_WASM_FN_CAP];
static unsigned char oo_wasm_pend[OO_WASM_PBUF];
static unsigned int oo_wasm_pend_len;
static int oo_wasm_have_pend;
static void oo_wasm_bind_print(unsigned int fi, const unsigned char *s,
                               unsigned int n) {
  unsigned int i, addnl, need;
  if (fi >= OO_WASM_FN_CAP || !s) return;
  addnl = (n == 0u || s[n - 1] != (unsigned char)'\n') ? 1u : 0u;
  need = n + addnl;
  if (need < 1u || oo_wasm_pused + need > OO_WASM_PBUF) {
    oo_wasm_ovf = 1;
    return;
  }
  oo_wasm_poff[fi] = oo_wasm_pused;
  for (i = 0; i < n; i++) oo_wasm_pbuf[oo_wasm_pused++] = s[i];
  if (addnl) oo_wasm_pbuf[oo_wasm_pused++] = (unsigned char)'\n';
  oo_wasm_plen[fi] = need;
}
static void oo_wasm_print_on_note(void) {
  if (!oo_wasm_have_pend || oo_wasm_name_n == 0u) return;
  oo_wasm_bind_print(oo_wasm_name_n - 1u, oo_wasm_pend, oo_wasm_pend_len);
  oo_wasm_have_pend = 0;
}
void oo_wasm_set_fn_print(OoStr s) {
  unsigned int n = 0, i;
  if (!s.data || s.len < 0) return;
  if (s.len > 0) {
    if (s.len > (long long)OO_WASM_PBUF) {
      oo_wasm_ovf = 1;
      return;
    }
    n = (unsigned int)s.len;
  } else {
    while (n < OO_WASM_PBUF && s.data[n]) n++;
    if (n == OO_WASM_PBUF && s.data[n]) {
      oo_wasm_ovf = 1;
      return;
    }
  }
  if (oo_wasm_name_n > 0u) {
    oo_wasm_bind_print(oo_wasm_name_n - 1u, (const unsigned char *)s.data, n);
    return;
  }
  for (i = 0; i < n; i++) oo_wasm_pend[i] = (unsigned char)s.data[i];
  oo_wasm_pend_len = n;
  oo_wasm_have_pend = 1;
}
static void oo_wasm_sleb(unsigned char *d, unsigned int *n, unsigned int v) {
  for (;;) {
    unsigned char b = (unsigned char)(v & 127u);
    v >>= 7;
    if (v != 0u || (b & 64u)) oo_wasm_u8(d, n, (unsigned int)b | 128u);
    else {
      oo_wasm_u8(d, n, b);
      return;
    }
  }
}
static void oo_wasm_i32c(unsigned char *d, unsigned int *n, unsigned int v) {
  oo_wasm_u8(d, n, 0x41);
  oo_wasm_sleb(d, n, v);
}
/* Overflow: no canned module. Caps: pay 24000, mod 28000, pbuf 4096 shared. */
static void oo_wasm_emit_tiny(void) {
  fputs("ERR\twasm\tbuf\n", stdout);
  fflush(stdout);
}
static void oo_wasm_emit_print_body(unsigned char *pay, unsigned int *psz,
                                    unsigned int off, unsigned int len) {
  unsigned char b[64];
  unsigned int bn = 0, save = oo_wasm_lim;
  int saveo = oo_wasm_ovf;
  oo_wasm_lim = 64u;
  oo_wasm_ovf = 0;
  oo_wasm_u8(b, &bn, 0);
  oo_wasm_i32c(b, &bn, 0);
  oo_wasm_i32c(b, &bn, OO_WASM_DATA + off);
  oo_wasm_u8(b, &bn, 0x36);
  oo_wasm_u8(b, &bn, 2);
  oo_wasm_u8(b, &bn, 0);
  oo_wasm_i32c(b, &bn, 4);
  oo_wasm_i32c(b, &bn, len);
  oo_wasm_u8(b, &bn, 0x36);
  oo_wasm_u8(b, &bn, 2);
  oo_wasm_u8(b, &bn, 0);
  oo_wasm_i32c(b, &bn, 1);
  oo_wasm_i32c(b, &bn, 0);
  oo_wasm_i32c(b, &bn, 1);
  oo_wasm_i32c(b, &bn, 8);
  oo_wasm_u8(b, &bn, 0x10);
  oo_wasm_u8(b, &bn, 0);
  oo_wasm_u8(b, &bn, 0x1a);
  oo_wasm_u8(b, &bn, 0x0b);
  if (oo_wasm_ovf) {
    oo_wasm_lim = save;
    return;
  }
  oo_wasm_lim = save;
  oo_wasm_ovf = saveo;
  oo_wasm_leb(pay, psz, bn);
  oo_wasm_raw(pay, psz, b, bn);
}
static void oo_wasm_emit_n(unsigned int n) {
  static unsigned char pay[24000], mod[28000];
  unsigned int i, psz, mlen, nlen, main_i = 0, base = 0;
  const char *nm;
  static const unsigned char hv[8] = {0x00, 0x61, 0x73, 0x6d,
                                      0x01, 0x00, 0x00, 0x00};
  static const unsigned char ty0[4] = {0x01, 0x60, 0x00, 0x00};
  static const unsigned char ty1[12] = {0x02, 0x60, 0x00, 0x00, 0x60, 0x04,
                                        0x7f, 0x7f, 0x7f, 0x7f, 0x01, 0x7f};
  static const unsigned char mem[3] = {0x01, 0x00, 0x01};
  if (n < 1) n = 1;
  if (n > OO_WASM_FN_CAP) n = OO_WASM_FN_CAP;
  if (oo_wasm_have_pend) {
    unsigned int fi = 0;
    for (i = 0; i < n && i < oo_wasm_name_n; i++) {
      nm = oo_wasm_name_at(i);
      if (nm[0] == 'm' && nm[1] == 'a' && nm[2] == 'i' && nm[3] == 'n' && !nm[4]) {
        fi = i;
        break;
      }
    }
    oo_wasm_bind_print(fi, oo_wasm_pend, oo_wasm_pend_len);
    oo_wasm_have_pend = 0;
  }
  for (i = 0; i < n; i++) {
    nm = oo_wasm_name_at(i);
    if (nm[0] == 'm' && nm[1] == 'a' && nm[2] == 'i' && nm[3] == 'n' && !nm[4])
      main_i = i;
    if (i < OO_WASM_FN_CAP && oo_wasm_plen[i]) base = 1u;
  }
  if (oo_wasm_ovf) {
    oo_wasm_emit_tiny();
    return;
  }
  oo_wasm_lim = OO_WASM_MOD_MAX;
  mlen = 0;
  oo_wasm_raw(mod, &mlen, hv, 8);
  oo_wasm_lim = OO_WASM_PAY_MAX;
  psz = 0;
  oo_wasm_u8(pay, &psz, 8);
  oo_wasm_raw(pay, &psz, "ooda-fns", 8);
  oo_wasm_leb(pay, &psz, n);
  for (i = 0; i < n; i++) {
    nm = oo_wasm_name_at(i);
    nlen = oo_wasm_slen(nm);
    oo_wasm_leb(pay, &psz, nlen);
    oo_wasm_raw(pay, &psz, nm, nlen);
  }
  oo_wasm_lim = OO_WASM_MOD_MAX;
  oo_wasm_sec(mod, &mlen, 0, pay, psz);
  if (base) oo_wasm_sec(mod, &mlen, 1, ty1, 12);
  else oo_wasm_sec(mod, &mlen, 1, ty0, 4);
  if (base) {
    oo_wasm_lim = OO_WASM_PAY_MAX;
    psz = 0;
    oo_wasm_leb(pay, &psz, 1);
    oo_wasm_leb(pay, &psz, 22);
    oo_wasm_raw(pay, &psz, "wasi_snapshot_preview1", 22);
    oo_wasm_leb(pay, &psz, 8);
    oo_wasm_raw(pay, &psz, "fd_write", 8);
    oo_wasm_u8(pay, &psz, 0);
    oo_wasm_leb(pay, &psz, 1);
    oo_wasm_lim = OO_WASM_MOD_MAX;
    oo_wasm_sec(mod, &mlen, 2, pay, psz);
  }
  oo_wasm_lim = OO_WASM_PAY_MAX;
  psz = 0;
  oo_wasm_leb(pay, &psz, n);
  for (i = 0; i < n; i++) oo_wasm_u8(pay, &psz, 0);
  oo_wasm_lim = OO_WASM_MOD_MAX;
  oo_wasm_sec(mod, &mlen, 3, pay, psz);
  oo_wasm_sec(mod, &mlen, 5, mem, 3);
  oo_wasm_lim = OO_WASM_PAY_MAX;
  psz = 0;
  oo_wasm_leb(pay, &psz, n + 2u);
  for (i = 0; i < n; i++) {
    nm = oo_wasm_name_at(i);
    nlen = oo_wasm_slen(nm);
    oo_wasm_leb(pay, &psz, nlen);
    oo_wasm_raw(pay, &psz, nm, nlen);
    oo_wasm_u8(pay, &psz, 0);
    oo_wasm_leb(pay, &psz, i + base);
  }
  oo_wasm_u8(pay, &psz, 6);
  oo_wasm_raw(pay, &psz, "_start", 6);
  oo_wasm_u8(pay, &psz, 0);
  oo_wasm_leb(pay, &psz, main_i + base);
  oo_wasm_u8(pay, &psz, 6);
  oo_wasm_raw(pay, &psz, "memory", 6);
  oo_wasm_u8(pay, &psz, 2);
  oo_wasm_u8(pay, &psz, 0);
  oo_wasm_lim = OO_WASM_MOD_MAX;
  oo_wasm_sec(mod, &mlen, 7, pay, psz);
  oo_wasm_lim = OO_WASM_PAY_MAX;
  psz = 0;
  oo_wasm_leb(pay, &psz, n);
  for (i = 0; i < n; i++) {
    if (base && i < OO_WASM_FN_CAP && oo_wasm_plen[i])
      oo_wasm_emit_print_body(pay, &psz, oo_wasm_poff[i], oo_wasm_plen[i]);
    else {
      oo_wasm_u8(pay, &psz, 2);
      oo_wasm_u8(pay, &psz, 0);
      oo_wasm_u8(pay, &psz, 0x0b);
    }
  }
  oo_wasm_lim = OO_WASM_MOD_MAX;
  oo_wasm_sec(mod, &mlen, 10, pay, psz);
  if (base && oo_wasm_pused > 0u) {
    oo_wasm_lim = OO_WASM_PAY_MAX;
    psz = 0;
    oo_wasm_leb(pay, &psz, 1);
    oo_wasm_u8(pay, &psz, 0);
    oo_wasm_i32c(pay, &psz, OO_WASM_DATA);
    oo_wasm_u8(pay, &psz, 0x0b);
    oo_wasm_leb(pay, &psz, oo_wasm_pused);
    oo_wasm_raw(pay, &psz, oo_wasm_pbuf, oo_wasm_pused);
    oo_wasm_lim = OO_WASM_MOD_MAX;
    oo_wasm_sec(mod, &mlen, 11, pay, psz);
  }
  if (oo_wasm_ovf) {
    oo_wasm_emit_tiny();
    return;
  }
  fwrite(mod, 1, mlen, stdout);
  fflush(stdout);
}
