#!/usr/bin/env bash
# job: shell composition safety — quote-break paths must not create injection markers
# in:  pure bin/ooda + oodac/oodac
# out: exit 0 if product check (and oodac build) reject injection; normal check still works
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODA" ]] || { echo "ERR_NO_OODA: $OODA" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: $OODAC" >&2; exit 1; }

# --- 1) product ooda check: double-quote break must NOT create marker ---
M1="$TMPDIR/ooda_shell_safety_marker_prod_$$"
rm -f "$M1"
set +e
"$OODA" check 'fixtures/int_main.oo"; touch '"$M1"'; echo "' \
  >"$TMPDIR/ss_prod.out" 2>"$TMPDIR/ss_prod.err"
set -e
if [[ -e "$M1" ]]; then
  bad "product check quote-break created marker $M1"
  rm -f "$M1"
else
  pass "product check quote-break no marker"
fi

# --- 2) product ooda check: dollar/backtick noise must NOT create marker ---
M2="$TMPDIR/ooda_shell_safety_marker_dq_$$"
rm -f "$M2"
set +e
"$OODA" check "fixtures/int_main.oo\$(touch $M2)" \
  >"$TMPDIR/ss_dq.out" 2>"$TMPDIR/ss_dq.err"
set -e
if [[ -e "$M2" ]]; then
  bad "product check \$(…) created marker"
  rm -f "$M2"
else
  pass "product check dollar-paren no marker"
fi

# --- 3) oodac build: quote-break path must NOT create marker ---
M3="$TMPDIR/ooda_shell_safety_marker_build_$$"
rm -f "$M3"
set +e
"$OODAC" build 'fixtures/int_main.oo"; touch '"$M3"'; echo "' \
  "$TMPDIR/ss_build_out_$$" >"$TMPDIR/ss_b.out" 2>"$TMPDIR/ss_b.err"
set -e
if [[ -e "$M3" ]]; then
  bad "oodac build quote-break created marker $M3"
  rm -f "$M3"
else
  pass "oodac build quote-break no marker"
fi

# --- 4) honest control: normal product check still works ---
set +e
"$OODA" check "$ROOT/fixtures/int_main.oo" >"$TMPDIR/ss_ok.out" 2>"$TMPDIR/ss_ok.err"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  bad "normal product check failed exit=$rc"
else
  pass "normal product check still works"
fi

if [[ $fail -ne 0 ]]; then
  echo "shell_safety_smoke: FAILED" >&2
  exit 1
fi
echo "shell_safety_smoke: PASSED"
exit 0
