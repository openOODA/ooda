#!/usr/bin/env bash
# M114 Ambient List Memory Quotas
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then OODAC="$ROOT/bootstrap/seed/oodac"
  else echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; fi
fi

FAIL="$ROOT/fixtures/list_quota_fail.oo"
PASS="$ROOT/fixtures/list_quota_pass.oo"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

echo "--- M114 list quota ---"

T_BIN="$TMPDIR/list_quota_fail.bin"
if "$OODAC" build "$FAIL" "$T_BIN" >/dev/null; then
  set +e
  out="$("$T_BIN" 2>&1)"
  ec=$?
  set -e
  if [[ $ec -ne 0 ]] && echo "$out" | grep -q "ambient List memory quota exceeded"; then
    pass "list_quota_fail.oo aborted correctly"
  else
    bad "list_quota_fail.oo did not abort with quota exceeded. ec=$ec out=$out"
  fi
else
  bad "failed to compile list_quota_fail.oo"
fi

T_BIN2="$TMPDIR/list_quota_pass.bin"
if "$OODAC" build "$PASS" "$T_BIN2" >/dev/null; then
  set +e
  out="$("$T_BIN2" 2>&1)"
  ec=$?
  set -e
  if [[ $ec -eq 0 ]]; then
    pass "list_quota_pass.oo succeeded with alloc_bytes"
  else
    bad "list_quota_pass.oo failed unexpectedly. ec=$ec out=$out"
  fi
else
  bad "failed to compile list_quota_pass.oo"
fi

exit $fail
