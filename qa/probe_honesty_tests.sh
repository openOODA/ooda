#!/usr/bin/env bash
# job: QA probe suite — Claims & Docs Honesty Pack (K1-K4) on compiled product binaries
# in:  bin/ooda, oodac/oodac
# out: exit 0 if all K1-K4 honesty probes pass
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
  elif [[ -x "$ROOT/dist/ooda-v0.182.1-alpha-linux-x86_64/oodac/oodac" ]]; then OODAC="$ROOT/dist/ooda-v0.182.1-alpha-linux-x86_64/oodac/oodac"
  fi
fi
export OODAC_BIN="$OODAC"

fail=0
pass() { echo "OK [K-PROBE] $*"; }
bad() { echo "FAIL [K-PROBE] $*" >&2; fail=1; }

[[ -x "$OODA" ]] || { echo "ERR_MISSING_OODA: $OODA" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_MISSING_OODAC: $OODAC" >&2; exit 1; }

# K1: --emit-llvm residual flag returns exit code 2 with clean ERR message
set +e
out1=$("$OODA" build "$ROOT/fixtures/chs_list_string.oo" --emit-llvm 2>&1)
rc1=$?
set -e
if [[ $rc1 -eq 2 ]] && echo "$out1" | grep -qE 'ERR.*--emit-llvm residual'; then
  pass "K1 --emit-llvm residual flag fail-closed (exit=2)"
else
  bad "K1 --emit-llvm residual flag (rc=$rc1, out=$out1)"
fi

# K2: --fuzz residual flag returns exit code 2 with clean ERR message
set +e
out2=$("$OODA" test "$ROOT/fixtures/chs_list_string.oo" --fuzz 2>&1)
rc2=$?
set -e
if [[ $rc2 -eq 2 ]] && echo "$out2" | grep -qE 'ERR.*--fuzz residual'; then
  pass "K2 --fuzz residual flag fail-closed (exit=2)"
else
  bad "K2 --fuzz residual flag (rc=$rc2, out=$out2)"
fi

# K3: --release residual flag returns exit code 2 with clean ERR message
set +e
out3=$("$OODA" build "$ROOT/fixtures/chs_list_string.oo" --release 2>&1)
rc3=$?
set -e
if [[ $rc3 -eq 2 ]] && echo "$out3" | grep -qE 'ERR.*--release residual'; then
  pass "K3 --release residual flag fail-closed (exit=2)"
else
  bad "K3 --release residual flag (rc=$rc3, out=$out3)"
fi

# K4: --target wasm and --backend llvm residual flags
set +e
out4_wasm=$("$OODA" build "$ROOT/fixtures/chs_list_string.oo" --target wasm 2>&1)
rc4_wasm=$?
set -e
if [[ $rc4_wasm -eq 2 ]] && echo "$out4_wasm" | grep -qE 'ERR.*target wasm residual'; then
  pass "K4 --target wasm residual flag fail-closed (exit=2)"
else
  bad "K4 --target wasm residual flag (rc=$rc4_wasm, out=$out4_wasm)"
fi

set +e
out4_backend=$("$OODAC" --backend llvm check "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc4_backend=$?
set -e
if [[ $rc4_backend -eq 2 ]] && echo "$out4_backend" | grep -qE 'ERR.*backend.*llvm residual'; then
  pass "K4 oodac --backend llvm residual flag fail-closed (exit=2)"
else
  bad "K4 oodac --backend llvm residual flag (rc=$rc4_backend, out=$out4_backend)"
fi

if [[ $fail -ne 0 ]]; then
  echo "probe_honesty_tests: FAILED" >&2
  exit 1
fi
echo "probe_honesty_tests: PASSED"
exit 0
