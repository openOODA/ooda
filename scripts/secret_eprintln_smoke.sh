#!/usr/bin/env bash
# Phase 2: Secret eprintln sink depth (M160)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

set +e
"$OODAC_BIN" check "$ROOT/fixtures/secret_eprintln_fail.oo" >"$TMPDIR/se_f.out" 2>"$TMPDIR/se_f.err"
frc=$?
set -e
if [[ $frc -ne 0 ]] && grep -qE $'ERR\tsecret' "$TMPDIR/se_f.out" "$TMPDIR/se_f.err" 2>/dev/null; then
  pass "eprintln SECRET refuse"
else
  bad "eprintln fail should refuse"
  cat "$TMPDIR/se_f.out" "$TMPDIR/se_f.err" | head -10
fi

set +e
"$OODAC_BIN" check "$ROOT/fixtures/secret_eprintln_pass.oo" >"$TMPDIR/se_p.out" 2>"$TMPDIR/se_p.err"
prc=$?
set -e
if [[ $prc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/se_p.out"; then
  pass "eprintln public pass"
else
  bad "eprintln pass should OK"
fi

# println secret still works (no regression)
set +e
"$OODAC_BIN" check "$ROOT/fixtures/secret_sink_fail.oo" >"$TMPDIR/se_old.out" 2>"$TMPDIR/se_old.err"
orc=$?
set -e
[[ $orc -ne 0 ]] && pass "println secret still refuse" || bad "println secret regression"

if [[ $fail -ne 0 ]]; then
  echo "secret_eprintln_smoke: FAILED" >&2
  exit 1
fi
echo "secret_eprintln_smoke: PASSED"
exit 0
