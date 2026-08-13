#!/usr/bin/env bash
# job: M10 pure Bool-domain fuzz pass + fail rails (no Python)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: $OODA" >&2
  exit 1
fi

set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_bool_domain.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_bool_pass.out" 2>"$TMPDIR/fuzz_bool_pass.err"
prc=$?
set -e
if [[ $prc -eq 0 ]] && grep -qiE 'pure bool domain|Fuzzer pure' "$TMPDIR/fuzz_bool_pass.out" "$TMPDIR/fuzz_bool_pass.err" 2>/dev/null; then
  pass "fuzz bool domain pass"
elif [[ $prc -eq 0 ]]; then
  pass "fuzz bool domain pass (rc=0)"
else
  bad "fuzz bool domain pass rc=$prc"
  head -20 "$TMPDIR/fuzz_bool_pass.err" "$TMPDIR/fuzz_bool_pass.out" 2>/dev/null || true
fi

set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_bool_fail.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_bool_fail.out" 2>"$TMPDIR/fuzz_bool_fail.err"
frc=$?
set -e
if [[ $frc -ne 0 ]]; then
  pass "fuzz bool fail rail (rc=$frc)"
else
  bad "fuzz bool fail rail expected non-zero"
fi

# Unsupported domain still fail-closed
set +e
timeout 30 "$OODA" test "$ROOT/fixtures/chs_list_string.oo" --fuzz 2 \
  >"$TMPDIR/fuzz_bool_ni.out" 2>"$TMPDIR/fuzz_bool_ni.err"
nrc=$?
set -e
if [[ $nrc -ne 0 ]] && grep -qiE 'FUZZ_DOMAIN|fail-closed|int\|bool|supports only' \
  "$TMPDIR/fuzz_bool_ni.out" "$TMPDIR/fuzz_bool_ni.err" 2>/dev/null; then
  pass "fuzz non-domain fail-closed"
elif [[ $nrc -ne 0 ]]; then
  pass "fuzz non-domain non-zero (rc=$nrc)"
else
  bad "fuzz non-domain should fail-closed"
fi

if [[ $fail -ne 0 ]]; then
  echo "fuzz_bool_smoke: FAILED" >&2
  exit 1
fi
echo "fuzz_bool_smoke: PASSED"
exit 0
