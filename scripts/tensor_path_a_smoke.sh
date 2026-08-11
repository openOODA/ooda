#!/usr/bin/env bash
# M166 path A — flat List[Int] tensor helpers + std/math/tensor.oo
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
FIX="$ROOT/fixtures/tensor_path_a.oo"
STD="$ROOT/std/math/tensor.oo"

[[ -f "$STD" ]] || { echo "ERR_NO_STD: $STD" >&2; exit 1; }
if grep -q 'tensor_new' "$STD" \
  && grep -q 'tensor_get' "$STD" \
  && grep -q 'tensor_set' "$STD" \
  && grep -qiE 'flat|row-major|List\[Int\]' "$STD"; then
  pass "std/math/tensor.oo path A helpers"
else
  bad "tensor.oo missing helpers or honesty"
fi
if grep -qiE 'residual|nested|List\[List|ndarray' "$STD"; then
  pass "tensor residual honesty (no nested List[List] product)"
else
  bad "tensor missing residual honesty"
fi

[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: $OODAC" >&2; exit 1; }

set +e
"$OODAC" check "$STD" >"$TMPDIR/tens_std.out" 2>"$TMPDIR/tens_std.err"
src=$?
set -e
if [[ $src -eq 0 ]] && grep -qE '^OK' "$TMPDIR/tens_std.out"; then
  pass "check std/math/tensor.oo"
else
  bad "check tensor.oo rc=$src"
  head -10 "$TMPDIR/tens_std.out" "$TMPDIR/tens_std.err" 2>/dev/null || true
fi

set +e
"$OODAC" check "$FIX" >"$TMPDIR/tens_ck.out" 2>"$TMPDIR/tens_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/tens_ck.out"; then
  pass "check tensor_path_a.oo"
else
  bad "check fixture rc=$ckrc"
  head -8 "$TMPDIR/tens_ck.out" "$TMPDIR/tens_ck.err" 2>/dev/null || true
fi

set +e
"$OODAC" emit-c "$FIX" >"$TMPDIR/tens.c" 2>"$TMPDIR/tens.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/tens.c" "$TMPDIR/tens.err" 2>/dev/null; then
  bad "emit-c tensor_path_a"
  head -12 "$TMPDIR/tens.err" "$TMPDIR/tens.c" 2>/dev/null || true
else
  pass "emit-c tensor_path_a"
  gcc "${RT[@]}" "$TMPDIR/tens.c" -o "$TMPDIR/tens.bin" 2>"$TMPDIR/tens.gcc" || {
    bad "gcc tensor"; head -10 "$TMPDIR/tens.gcc" || true
  }
  if [[ -x "$TMPDIR/tens.bin" ]]; then
    out=$("$TMPDIR/tens.bin" 2>&1) || true
    # 7, 9, 0, 6
    if echo "$out" | tr '\n' ' ' | grep -qE '7.*9.*0.*6'; then
      pass "runtime flat tensor get/set rebuild"
    else
      bad "runtime out=$out"
    fi
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "tensor_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "tensor_path_a_smoke: PASSED"
exit 0
