#!/usr/bin/env bash
# job: import load honesty rails (in-tree check + residual concat/pure_build)
# in:  oodac binary + bootstrap/corpus/import
# out: exit 0 if pass/fail fixtures hold
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
PASS_DIR="$ROOT/bootstrap/corpus/import/pass"
FAIL_DIR="$ROOT/bootstrap/corpus/import/fail"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- multi-file check pass (in-tree load) ---
set +e
"$OODAC" check "$PASS_DIR/multi_ok.oo" >"$TMPDIR/imp_ok.out" 2>"$TMPDIR/imp_ok.err"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  bad "check multi_ok exit=$rc"
  cat "$TMPDIR/imp_ok.out" "$TMPDIR/imp_ok.err" | head -20 || true
elif ! grep -q '^OK' "$TMPDIR/imp_ok.out" 2>/dev/null; then
  bad "check multi_ok missing OK"
  cat "$TMPDIR/imp_ok.out" "$TMPDIR/imp_ok.err" | head -20 || true
else
  pass "check multi_ok (in-tree load)"
fi

# --- digit-0 module name (R2: false NUL reject regression) ---
set +e
"$OODAC" check "$PASS_DIR/multi_digit0.oo" >"$TMPDIR/imp_d0.out" 2>"$TMPDIR/imp_d0.err"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  bad "check multi_digit0 exit=$rc"
  cat "$TMPDIR/imp_d0.out" "$TMPDIR/imp_d0.err" | head -20 || true
elif ! grep -q '^OK' "$TMPDIR/imp_d0.out" 2>/dev/null; then
  bad "check multi_digit0 missing OK"
  cat "$TMPDIR/imp_d0.out" "$TMPDIR/imp_d0.err" | head -20 || true
elif grep -q 'NUL' "$TMPDIR/imp_d0.out" "$TMPDIR/imp_d0.err" 2>/dev/null; then
  bad "check multi_digit0 false NUL reject"
else
  pass "check multi_digit0 (digit-0 module name)"
fi

# --- missing import fail-closed ---
set +e
"$OODAC" check "$FAIL_DIR/missing.oo" >"$TMPDIR/imp_miss.out" 2>"$TMPDIR/imp_miss.err"
rc=$?
set -e
out=$(cat "$TMPDIR/imp_miss.out" "$TMPDIR/imp_miss.err" 2>/dev/null || true)
if [[ $rc -eq 0 ]]; then
  bad "check missing accepted"
elif ! echo "$out" | grep -qE 'ERR[[:space:]]+import|ERR_IMPORT_MISSING|missing'; then
  bad "check missing missing ERR import (got: $out)"
else
  pass "check missing fail-closed"
fi

# --- cycle fail-closed ---
set +e
"$OODAC" check "$FAIL_DIR/cycle_a.oo" >"$TMPDIR/imp_cyc.out" 2>"$TMPDIR/imp_cyc.err"
rc=$?
set -e
out=$(cat "$TMPDIR/imp_cyc.out" "$TMPDIR/imp_cyc.err" 2>/dev/null || true)
if [[ $rc -eq 0 ]]; then
  bad "check cycle accepted"
elif ! echo "$out" | grep -qE 'ERR[[:space:]]+import|ERR_IMPORT_CYCLE|cycle'; then
  bad "check cycle missing ERR import (got: $out)"
else
  pass "check cycle fail-closed"
fi

# --- residual concat: missing + cycle ---
set +e
sh "$ROOT/scripts/oodac_concat.sh" "$FAIL_DIR/missing.oo" >"$TMPDIR/cat_miss.out" 2>"$TMPDIR/cat_miss.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "concat missing accepted"
elif ! grep -qE 'ERR_IMPORT_MISSING|missing' "$TMPDIR/cat_miss.err" 2>/dev/null; then
  bad "concat missing no ERR_IMPORT_MISSING"
else
  pass "concat missing residual fail-closed"
fi

set +e
sh "$ROOT/scripts/oodac_concat.sh" "$FAIL_DIR/cycle_a.oo" >"$TMPDIR/cat_cyc.out" 2>"$TMPDIR/cat_cyc.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "concat cycle accepted"
elif ! grep -qE 'ERR_IMPORT_CYCLE|cycle' "$TMPDIR/cat_cyc.err" 2>/dev/null; then
  bad "concat cycle no ERR_IMPORT_CYCLE"
else
  pass "concat cycle residual fail-closed"
fi

# --- pure_build missing (product build path residual) ---
set +e
OODAC_BIN="$OODAC" sh "$ROOT/scripts/oodac_pure_build.sh" "$FAIL_DIR/missing.oo" "$TMPDIR/imp_miss.bin" \
  >"$TMPDIR/pb_miss.out" 2>"$TMPDIR/pb_miss.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "pure_build missing accepted"
elif ! grep -qE 'ERR_MISSING|ERR_IMPORT' "$TMPDIR/pb_miss.err" "$TMPDIR/pb_miss.out" 2>/dev/null; then
  bad "pure_build missing no ERR"
  cat "$TMPDIR/pb_miss.out" "$TMPDIR/pb_miss.err" | head -10 || true
else
  pass "pure_build missing fail-closed"
fi

set +e
OODAC_BIN="$OODAC" sh "$ROOT/scripts/oodac_pure_build.sh" "$FAIL_DIR/cycle_a.oo" "$TMPDIR/imp_cyc.bin" \
  >"$TMPDIR/pb_cyc.out" 2>"$TMPDIR/pb_cyc.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "pure_build cycle accepted"
elif ! grep -qE 'ERR_IMPORT_CYCLE|cycle' "$TMPDIR/pb_cyc.err" "$TMPDIR/pb_cyc.out" 2>/dev/null; then
  bad "pure_build cycle no ERR_IMPORT_CYCLE"
  cat "$TMPDIR/pb_cyc.out" "$TMPDIR/pb_cyc.err" | head -10 || true
else
  pass "pure_build cycle fail-closed"
fi

if [[ $fail -ne 0 ]]; then
  echo "import_load_smoke: FAILED" >&2
  exit 1
fi
echo "import_load_smoke: ALL OK"
exit 0
