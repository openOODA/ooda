#!/usr/bin/env bash
# M125 Ambient List memory quotas — fail-closed + AllocCap raise
# Bounded: small OO_LIST_AMBIENT_QUOTA + List[Int] (no multi-minute hang)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA_BIN:-$ROOT/bin/ooda}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODA" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then
    # product CLI preferred; fall back to pure build path via ooda_product
    OODA="$ROOT/bin/ooda"
  fi
fi
[[ -x "$OODA" ]] || { echo "ERR_NO_OODA: need bin/ooda" >&2; exit 1; }

FAIL="$ROOT/fixtures/list_quota_fail.oo"
PASS="$ROOT/fixtures/list_quota_pass.oo"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

echo "--- M125 list quota ---"

# Fail: build under default quota (compiler needs headroom); run with tiny budget
T_BIN="$TMPDIR/list_quota_fail.bin"
rm -f "$T_BIN"
set +e
env -u OO_LIST_AMBIENT_QUOTA "$OODA" build "$FAIL" -o "$T_BIN" >/dev/null 2>"$TMPDIR/lq_f_build.err"
brc=$?
set -e
if [[ $brc -ne 0 || ! -x "$T_BIN" ]]; then
  bad "list_quota_fail.oo failed to build brc=$brc"
  cat "$TMPDIR/lq_f_build.err" >&2 || true
else
  set +e
  out="$(OO_LIST_AMBIENT_QUOTA=256 timeout 5 "$T_BIN" 2>&1)"
  ec=$?
  set -e
  if [[ $ec -ne 0 ]] && echo "$out" | grep -q "ambient List memory quota exceeded"; then
    pass "list_quota_fail.oo aborted with quota exceeded (ec=$ec)"
  else
    bad "list_quota_fail.oo did not hit quota. ec=$ec out=$out"
  fi
fi

# Pass: run with tiny ambient + AllocCap raise inside program
T_BIN2="$TMPDIR/list_quota_pass.bin"
rm -f "$T_BIN2"
set +e
env -u OO_LIST_AMBIENT_QUOTA "$OODA" build "$PASS" -o "$T_BIN2" >/dev/null 2>"$TMPDIR/lq_p_build.err"
brc2=$?
set -e
if [[ $brc2 -ne 0 || ! -x "$T_BIN2" ]]; then
  bad "list_quota_pass.oo failed to build brc=$brc2"
  cat "$TMPDIR/lq_p_build.err" >&2 || true
else
  set +e
  out="$(OO_LIST_AMBIENT_QUOTA=256 timeout 5 "$T_BIN2" 2>&1)"
  ec=$?
  set -e
  if [[ $ec -eq 0 ]] && echo "$out" | grep -qE '50'; then
    pass "list_quota_pass.oo under raised ceiling (out=$out)"
  else
    bad "list_quota_pass.oo unexpected. ec=$ec out=$out"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "list_quota_smoke: FAILED" >&2
  exit 1
fi
echo "list_quota_smoke: PASSED"
exit 0
