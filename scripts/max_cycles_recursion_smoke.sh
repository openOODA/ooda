#!/usr/bin/env bash
# M126 MaxCycles recursion fuel — shared static __oo_mc debits call entry
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA_BIN:-$ROOT/bin/ooda}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"

[[ -x "$OODA" ]] || { echo "ERR_NO_OODA" >&2; exit 1; }
# Product CLI resolves scripts relative to cwd
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
PASS="$ROOT/fixtures/max_cycles_recursion_pass.oo"
FAIL="$ROOT/fixtures/max_cycles_recursion_fail.oo"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

echo "--- M126 max_cycles recursion ---"

PBIN="$TMPDIR/mc_rec_pass.bin"
FBIN="$TMPDIR/mc_rec_fail.bin"
rm -f "$PBIN" "$FBIN"

set +e
"$OODA" build "$PASS" -o "$PBIN" >/dev/null 2>"$TMPDIR/mc_rp.err"
prc=$?
set -e
if [[ $prc -ne 0 || ! -x "$PBIN" ]]; then
  bad "recursion_pass build failed"
  cat "$TMPDIR/mc_rp.err" >&2 || true
else
  set +e
  out="$(timeout 5 "$PBIN" 2>&1)"
  ec=$?
  set -e
  if [[ $ec -eq 0 ]]; then
    pass "recursion_pass ran under budget (out=$out)"
  else
    bad "recursion_pass failed ec=$ec out=$out"
  fi
fi

set +e
"$OODA" build "$FAIL" -o "$FBIN" >/dev/null 2>"$TMPDIR/mc_rf.err"
frc=$?
set -e
if [[ $frc -ne 0 || ! -x "$FBIN" ]]; then
  bad "recursion_fail build failed"
  cat "$TMPDIR/mc_rf.err" >&2 || true
else
  set +e
  out="$(timeout 5 "$FBIN" 2>&1)"
  ec=$?
  set -e
  if [[ $ec -ne 0 ]] && echo "$out" | grep -qE $'ERR\tmax_cycles\texceeded|max_cycles'; then
    pass "recursion_fail exceeded fuel (ec=$ec)"
  else
    bad "recursion_fail did not exceed. ec=$ec out=$out"
  fi
fi

# Native emit must use static shared counter (not per-fn reset)
set +e
raw="$("$ROOT/oodac/oodac" emit-c "$FAIL" 2>/dev/null || true)"
set -e
if echo "$raw" | grep -q 'static long long __oo_mc'; then
  pass "emit has static shared __oo_mc"
else
  bad "emit missing static long long __oo_mc"
fi
# Per-fn local reset would be bare `long long __oo_mc = 0` without static
if echo "$raw" | grep -E '^[[:space:]]*long long __oo_mc = 0;' >/dev/null 2>&1; then
  bad "emit still has per-fn local __oo_mc = 0"
else
  pass "no per-fn local __oo_mc reset"
fi
if echo "$raw" | grep -q 'OO_MC_LIMIT'; then
  pass "emit has OO_MC_LIMIT macro"
else
  bad "emit missing OO_MC_LIMIT"
fi

if [[ $fail -ne 0 ]]; then
  echo "max_cycles_recursion_smoke: FAILED" >&2
  exit 1
fi
echo "max_cycles_recursion_smoke: PASSED"
exit 0
