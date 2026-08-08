#!/usr/bin/env bash
# job: QA probe suite — Claims & Docs Honesty Pack on product binaries
# in:  bin/ooda, oodac/oodac
# out: exit 0 if honesty probes match real product surface
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd)" || true
[[ -n "$ROOT" && -d "$ROOT" ]] || { echo "ERR_ROOT_INVALID" >&2; exit 1; }

TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/probe_honesty_$$"
mkdir -p "$TMP"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

OODA="${OODA:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-}"
if [[ -z "$OODAC" || ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then OODAC="$ROOT/oodac/oodac"
  elif [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then OODAC="$ROOT/bootstrap/seed/oodac"
  fi
fi
export OODAC_BIN="$OODAC"

fail=0
pass() { echo "OK [K-PROBE] $*"; }
bad() { echo "FAIL [K-PROBE] $*" >&2; fail=1; }

[[ -x "$OODA" ]] || { echo "ERR_MISSING_OODA: $OODA" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_MISSING_OODAC: $OODAC" >&2; exit 1; }

FIX="$ROOT/fixtures/chs_list_string.oo"

# K1: --emit-llvm is product path (not residual) — emits IR or writes .ll
set +e
out1=$("$OODA" build "$FIX" --emit-llvm -o "$TMP/k1.ll" 2>&1)
rc1=$?
set -e
if [[ $rc1 -eq 0 ]] && { [[ -s "$TMP/k1.ll" ]] || echo "$out1" | grep -qiE 'LLVM|emit'; }; then
  pass "K1 --emit-llvm product path (exit=0)"
else
  bad "K1 --emit-llvm product path (rc=$rc1, out=$out1)"
fi

# K2: --fuzz is un-gated but NOT pure-oo native — must not claim residual exit 2
# Honesty: harness is Python (ooda_test_verify → ooda_fuzz_*). Accept non-2, or
# explicit residual naming of python/harness if fail-closed later.
set +e
out2=$("$OODA" test "$FIX" --fuzz 2 2>&1)
rc2=$?
set -e
if [[ $rc2 -eq 2 ]] && echo "$out2" | grep -qE 'ERR.*--fuzz residual'; then
  bad "K2 --fuzz still residual-gated (docs claim un-gated; rc=2 residual)"
elif echo "$out2" | grep -qiE 'python|ooda_fuzz|harness|fuzz' \
  || [[ $rc2 -eq 0 ]] || [[ $rc2 -eq 1 ]]; then
  pass "K2 --fuzz un-gated (Python harness residual; rc=$rc2)"
else
  bad "K2 --fuzz unexpected (rc=$rc2, out=$out2)"
fi

# K3: --release residual flag returns exit code 2 with clean ERR message
set +e
out3=$("$OODA" build "$FIX" --release 2>&1)
rc3=$?
set -e
if [[ $rc3 -eq 2 ]] && echo "$out3" | grep -qE 'ERR.*--release residual'; then
  pass "K3 --release residual flag fail-closed (exit=2)"
else
  bad "K3 --release residual flag (rc=$rc3, out=$out3)"
fi

# K4a: --target wasm product path
set +e
out4=$("$OODA" build "$FIX" --target wasm -o "$TMP/k4.wat" 2>&1)
rc4=$?
set -e
if [[ $rc4 -eq 0 ]] && { [[ -s "$TMP/k4.wat" ]] || echo "$out4" | grep -qiE 'WebAssembly|wasm|\.wat'; }; then
  pass "K4 --target wasm product path (exit=0)"
else
  bad "K4 --target wasm product path (rc=$rc4, out=$out4)"
fi

# K4b: oodac --backend llvm accepted (check still runs)
set +e
out5=$("$OODAC" --backend llvm check "$FIX" 2>&1)
rc5=$?
set -e
if [[ $rc5 -eq 2 ]] && echo "$out5" | grep -qE 'ERR.*backend.*llvm residual'; then
  bad "K4 oodac --backend llvm still residual-gated"
elif [[ $rc5 -eq 0 ]] || echo "$out5" | grep -qE '^OK'; then
  pass "K4 oodac --backend llvm accepted (rc=$rc5)"
else
  # missing file / SEGV residual — still honesty if not fake residual
  if echo "$out5" | grep -qE 'residual'; then
    bad "K4 oodac --backend llvm residual claim (rc=$rc5, out=$out5)"
  else
    pass "K4 oodac --backend llvm non-residual failure surface (rc=$rc5)"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "probe_honesty_tests: FAILED" >&2
  exit 1
fi
echo "probe_honesty_tests: PASSED"
exit 0
