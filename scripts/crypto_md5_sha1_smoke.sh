#!/usr/bin/env bash
# Path A crypto floor: real MD5/SHA-1 hex digests; AES residual STUB_FAIL_CLOSED
# in:  oodac + runtime/chs_rt (hash) + fixtures/crypto_md5_sha1_vectors.oo
# out: exit 0 if known vectors match and std wrappers check
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)

# Expected vectors (NIST / common test suite)
MD5_EMPTY="d41d8cd98f00b204e9800998ecf8427e"
MD5_HELLO="5d41402abc4b2a76b9719d911017c592"
SHA1_EMPTY="da39a3ee5e6b4b0d3255bfef95601890afd80709"
AES_STUB="STUB_FAIL_CLOSED"

FIX="$ROOT/fixtures/crypto_md5_sha1_vectors.oo"
[[ -f "$FIX" ]] || bad "missing fixture $FIX"

# std wrappers check (library, no main required)
for m in "$ROOT/std/hash/md5.oo" "$ROOT/std/hash/sha1.oo" "$ROOT/std/core/crypto.oo"; do
  base="$(basename "$m")"
  set +e
  "$OODAC" check "$m" >"$TMPDIR/ck_$base.out" 2>"$TMPDIR/ck_$base.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/ck_$base.out" "$TMPDIR/ck_$base.err" 2>/dev/null; then
    bad "check $base"
    head -8 "$TMPDIR/ck_$base.err" "$TMPDIR/ck_$base.out" 2>/dev/null || true
  else
    pass "check $base"
  fi
done

# free-name vector fixture: emit-c + link chs_rt + run
set +e
"$OODAC" emit-c "$FIX" >"$TMPDIR/crypto_vec.c" 2>"$TMPDIR/crypto_vec.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/crypto_vec.c" "$TMPDIR/crypto_vec.err" 2>/dev/null; then
  bad "emit-c crypto_md5_sha1_vectors"
  head -20 "$TMPDIR/crypto_vec.err" "$TMPDIR/crypto_vec.c" 2>/dev/null || true
else
  pass "emit-c crypto_md5_sha1_vectors"
  # honesty: C must call real internals (not a soft-pass stub of digests)
  grep -q 'crypto_md5_internal' "$TMPDIR/crypto_vec.c" && pass "emit lowers crypto_md5_internal" \
    || bad "emit missing crypto_md5_internal"
  grep -q 'crypto_sha1_internal' "$TMPDIR/crypto_vec.c" && pass "emit lowers crypto_sha1_internal" \
    || bad "emit missing crypto_sha1_internal"
  grep -q 'crypto_aes_encrypt_internal' "$TMPDIR/crypto_vec.c" && pass "emit lowers crypto_aes_encrypt_internal" \
    || bad "emit missing crypto_aes_encrypt_internal"

  if ! gcc "${RT[@]}" "$TMPDIR/crypto_vec.c" -o "$TMPDIR/crypto_vec.bin" 2>"$TMPDIR/crypto_vec.gcc"; then
    bad "gcc link crypto vectors"
    head -20 "$TMPDIR/crypto_vec.gcc" || true
  else
    set +e
    out=$("$TMPDIR/crypto_vec.bin" 2>&1)
    rrc=$?
    set -e
    if [[ $rrc -ne 0 ]]; then
      bad "run vectors rc=$rrc out=$out"
    else
      mapfile -t lines <<<"$out"
      got_md5e="${lines[0]:-}"
      got_md5h="${lines[1]:-}"
      got_sha1="${lines[2]:-}"
      got_aes="${lines[3]:-}"
      [[ "$got_md5e" == "$MD5_EMPTY" ]] && pass "MD5(\"\")=$MD5_EMPTY" \
        || bad "MD5(\"\") got=$got_md5e want=$MD5_EMPTY"
      [[ "$got_md5h" == "$MD5_HELLO" ]] && pass "MD5(\"hello\")=$MD5_HELLO" \
        || bad "MD5(\"hello\") got=$got_md5h want=$MD5_HELLO"
      [[ "$got_sha1" == "$SHA1_EMPTY" ]] && pass "SHA1(\"\")=$SHA1_EMPTY" \
        || bad "SHA1(\"\") got=$got_sha1 want=$SHA1_EMPTY"
      [[ "$got_aes" == "$AES_STUB" ]] && pass "AES residual=$AES_STUB" \
        || bad "AES residual got=$got_aes want=$AES_STUB"
    fi
  fi
fi

# residual honesty: AES must not claim real ciphertext
if grep -q 'STUB_FAIL_CLOSED' "$ROOT/runtime/chs_rt_hash.c"; then
  pass "runtime AES residual string present"
else
  bad "runtime AES residual missing STUB_FAIL_CLOSED"
fi

if [[ $fail -ne 0 ]]; then
  echo "crypto_md5_sha1_smoke: FAILED" >&2
  exit 1
fi
echo "crypto_md5_sha1_smoke: PASSED"
exit 0
