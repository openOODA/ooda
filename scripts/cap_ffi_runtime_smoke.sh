#!/usr/bin/env bash
# Cap vs FFI runtime seal (M156) — process-local UnsafeFFICap + oo_dlopen stub
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)

# check still refuses bare dlopen
set +e
"$OODAC_BIN" check "$ROOT/fixtures/ffi_dlopen_fail.oo" >"$TMPDIR/cfr_bare.out" 2>"$TMPDIR/cfr_bare.err"
brc=$?
set -e
[[ $brc -ne 0 ]] && pass "check refuse bare dlopen" || bad "bare dlopen accepted"

# emit + run with grant
FIX="$ROOT/fixtures/ffi_dlopen_runtime_pass.oo"
set +e
"$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/cfr_pass.c" 2>"$TMPDIR/cfr_pass.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cfr_pass.c" "$TMPDIR/cfr_pass.err" 2>/dev/null; then
  bad "emit-c runtime pass"
  head -20 "$TMPDIR/cfr_pass.err" "$TMPDIR/cfr_pass.c" || true
else
  pass "emit-c dlopen with UnsafeFFICap"
fi
if grep -q 'oo_dlopen' "$TMPDIR/cfr_pass.c" && grep -q 'oo_cap_grant_ffi' "$TMPDIR/cfr_pass.c"; then
  pass "emit lowers oo_dlopen + grant_ffi"
else
  bad "missing oo_dlopen/grant_ffi in emit"
  grep -n 'dlopen\|ffi\|grant' "$TMPDIR/cfr_pass.c" | head -20 || true
fi

gcc "${RT[@]}" "$TMPDIR/cfr_pass.c" -o "$TMPDIR/cfr_pass.bin" 2>"$TMPDIR/cfr_gcc.err" || {
  bad "gcc link"
  cat "$TMPDIR/cfr_gcc.err" | head -20
}
if [[ -x "$TMPDIR/cfr_pass.bin" ]]; then
  out=$("$TMPDIR/cfr_pass.bin" 2>&1) || true
  if echo "$out" | grep -q 'ffi-stub-ok'; then
    pass "runtime stub dlopen with real grant"
  else
    bad "runtime out=$out"
  fi
  # forge deny: zero token
  sed -E 's/long long ffi = oo_cap_grant_ffi\(\)/long long ffi = 0LL/' \
    "$TMPDIR/cfr_pass.c" >"$TMPDIR/cfr_zero.c"
  gcc "${RT[@]}" "$TMPDIR/cfr_zero.c" -o "$TMPDIR/cfr_zero.bin" -lm -ldl -lpthread
  set +e
  zout=$("$TMPDIR/cfr_zero.bin" 2>&1); zrc=$?
  set -e
  if [[ $zrc -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
    pass "zero forge deny"
  else
    bad "zero forge not denied out=$zout rc=$zrc"
  fi
  # classic magic forge
  sed -E 's/long long ffi = oo_cap_grant_ffi\(\)/long long ffi = 0x4F4F4649LL/' \
    "$TMPDIR/cfr_pass.c" >"$TMPDIR/cfr_mag.c"
  gcc "${RT[@]}" "$TMPDIR/cfr_mag.c" -o "$TMPDIR/cfr_mag.bin" -lm -ldl -lpthread
  set +e
  mout=$("$TMPDIR/cfr_mag.bin" 2>&1); mrc=$?
  set -e
  if [[ $mrc -ne 0 ]] && echo "$mout" | grep -qE $'ERR[\t ]*cap'; then
    pass "magic forge deny"
  else
    bad "magic forge not denied out=$mout rc=$mrc"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_ffi_runtime_smoke: FAILED" >&2
  exit 1
fi
echo "cap_ffi_runtime_smoke: PASSED"
exit 0
