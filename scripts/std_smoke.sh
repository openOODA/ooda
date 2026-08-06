#!/usr/bin/env bash
# job: pure std modules check + multi-build fixtures
# in:  oodac, std/, fixtures/std_*_main.oo
# out: exit 0 if std is real on pure path
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo ERR_NO_OODAC >&2; exit 1; }

for m in result.oo str.oo option.oo; do
  set +e
  "$OODAC" check "$ROOT/std/$m" >"$TMPDIR/std_ck_$m.out" 2>"$TMPDIR/std_ck_$m.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then bad "check std/$m"; else pass "check std/$m"; fi
done

for f in std_result_main.oo std_str_main.oo std_option_main.oo; do
  set +e
  OODAC_BIN="$OODAC" "$OODAC" build "$ROOT/fixtures/$f" "$TMPDIR/${f%.oo}" \
    >"$TMPDIR/std_b_$f.out" 2>"$TMPDIR/std_b_$f.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 || ! -x "$TMPDIR/${f%.oo}" ]]; then
    bad "build fixtures/$f"
    head -8 "$TMPDIR/std_b_$f.err" 2>/dev/null || true
  else
    set +e
    "$TMPDIR/${f%.oo}" >"$TMPDIR/std_r_$f.out" 2>"$TMPDIR/std_r_$f.err"
    rr=$?
    set -e
    if [[ $rr -ne 0 ]]; then bad "run fixtures/$f"; else pass "build+run fixtures/$f"; fi
  fi
done

if [[ $fail -ne 0 ]]; then
  echo "std_smoke: FAILED" >&2
  exit 1
fi
echo "std_smoke: PASSED"
exit 0
