#!/usr/bin/env bash
# job: verify data-oriented design and struct-of-arrays engine
# in:  oodac, std/src/ooda/dod/*.oo, fixtures/dod_fixture.oo
# out: exit 0 if DoD/SoA modules typecheck, compile, and execute cleanly
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PARENT="$(cd "$ROOT/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo ERR_NO_OODAC >&2; exit 1; }

# 1. Typecheck standard library DoD and SoA modules
for m in \
  "$PARENT/std/src/ooda/dod/soa_layout_types.oo" \
  "$PARENT/std/src/ooda/dod/soa_layout_ops.oo" \
  "$PARENT/std/src/ooda/dod/soa_layout.oo" \
  "$PARENT/std/src/ooda/dod/dod_layout_types.oo" \
  "$PARENT/std/src/ooda/dod/dod_layout_ops.oo" \
  "$PARENT/std/src/ooda/dod/dod_layout.oo"; do
  set +e
  "$OODAC" check "$m" >"$TMPDIR/dod_ck_$(basename "$m").out" 2>"$TMPDIR/dod_ck_$(basename "$m").err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    bad "check $(basename "$m")"
    cat "$TMPDIR/dod_ck_$(basename "$m").err" 2>/dev/null || true
  else
    pass "check $(basename "$m")"
  fi
done

# 2. Typecheck, build, and run fixture
set +e
"$OODAC" check "$ROOT/fixtures/dod_fixture.oo" >"$TMPDIR/dod_fix_ck.out" 2>"$TMPDIR/dod_fix_ck.err"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then bad "check fixtures/dod_fixture.oo"; else pass "check fixtures/dod_fixture.oo"; fi

set +e
OODAC_BIN="$OODAC" "$OODAC" build "$ROOT/fixtures/dod_fixture.oo" "$TMPDIR/dod_fixture" \
  >"$TMPDIR/dod_b_fix.out" 2>"$TMPDIR/dod_b_fix.err"
rc=$?
set -e
if [[ $rc -ne 0 || ! -x "$TMPDIR/dod_fixture" ]]; then
  bad "build fixtures/dod_fixture.oo"
  cat "$TMPDIR/dod_b_fix.err" 2>/dev/null || true
else
  set +e
  "$TMPDIR/dod_fixture" >"$TMPDIR/dod_r_fix.out" 2>"$TMPDIR/dod_r_fix.err"
  rr=$?
  set -e
  if [[ $rr -ne 0 ]] || ! grep -q "dod_fixture: PASSED" "$TMPDIR/dod_r_fix.out"; then
    bad "run fixtures/dod_fixture.oo"
  else
    pass "build+run fixtures/dod_fixture.oo"
  fi
fi

# 3. Verify DINNER.oot 4-element headers and <= 256 lines
for f in \
  "$PARENT/std/src/ooda/dod/soa_layout_types.oo" \
  "$PARENT/std/src/ooda/dod/soa_layout_ops.oo" \
  "$PARENT/std/src/ooda/dod/soa_layout.oo" \
  "$PARENT/std/src/ooda/dod/dod_layout_types.oo" \
  "$PARENT/std/src/ooda/dod/dod_layout_ops.oo" \
  "$PARENT/std/src/ooda/dod/dod_layout.oo" \
  "$ROOT/fixtures/dod_fixture.oo" \
  "$ROOT/scripts/dod_soa_smoke.oo"; do
  lines=$(wc -l < "$f")
  if [[ $lines -gt 256 ]]; then
    bad "line limit $f ($lines > 256)"
  else
    pass "line count $f <= 256 ($lines lines)"
  fi
  if ! head -n 15 "$f" | grep -q "^// # "; then bad "missing title $f"; fi
  if ! head -n 15 "$f" | grep -q "^// Logline:"; then bad "missing logline $f"; fi
  if ! head -n 15 "$f" | grep -q "^// Setup:"; then bad "missing setup $f"; fi
  if ! head -n 15 "$f" | grep -q "^// Beats:"; then bad "missing beats $f"; fi
done

if [[ $fail -ne 0 ]]; then
  echo "dod_soa_smoke: FAILED" >&2
  exit 1
fi
echo "dod_soa_smoke: PASSED"
exit 0
