#!/usr/bin/env bash
# job: M3 pure Int-domain depth rails — add/mul/abs/clamp pass + fail rail
# in:  bin/ooda or oodac product path; fixtures/fuzz_int_*.oo
# out: exit 0 if all depth fixtures behave as expected
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
if [[ ! -x "$OODA" ]]; then
  # Fall back to product_sh path via tree ooda if present
  if [[ -x "$ROOT/bin/ooda" ]]; then OODA="$ROOT/bin/ooda"; fi
fi
if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: $OODA" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

run_pass() {
  local name="$1" src="$2"
  local out="$TMPDIR/fuzz_depth_${name}.out" err="$TMPDIR/fuzz_depth_${name}.err"
  set +e
  timeout 60 "$OODA" test "$src" --fuzz 20 >"$out" 2>"$err"
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    pass "fuzz depth pass $name"
  else
    bad "fuzz depth pass $name rc=$rc"
    head -20 "$err" "$out" 2>/dev/null || true
  fi
}

run_fail() {
  local name="$1" src="$2"
  local out="$TMPDIR/fuzz_depth_${name}.out" err="$TMPDIR/fuzz_depth_${name}.err"
  set +e
  timeout 60 "$OODA" test "$src" --fuzz 20 >"$out" 2>"$err"
  local rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    pass "fuzz depth fail-rail $name (rc=$rc)"
  else
    bad "fuzz depth fail-rail $name expected non-zero"
    head -20 "$out" 2>/dev/null || true
  fi
}

run_pass "add" "$ROOT/fixtures/fuzz_int_add.oo"
run_pass "mul" "$ROOT/fixtures/fuzz_int_mul.oo"
run_pass "abs" "$ROOT/fixtures/fuzz_int_abs.oo"
run_pass "clamp" "$ROOT/fixtures/fuzz_int_clamp.oo"
run_pass "domain" "$ROOT/fixtures/fuzz_int_domain.oo"
run_fail "fail" "$ROOT/fixtures/fuzz_int_fail.oo"

if [[ $fail -ne 0 ]]; then
  echo "fuzz_int_depth_smoke: FAILED" >&2
  exit 1
fi
echo "fuzz_int_depth_smoke: PASSED"
exit 0
