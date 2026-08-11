#!/usr/bin/env bash
# Secret path A product floor (alpha)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
run() {
  echo "=== $1 ==="
  if bash "$ROOT/scripts/$1"; then
    echo "OK $1"
  else
    echo "FAIL $1" >&2
    fail=1
  fi
}
run secret_sink_enforce_smoke.sh
run secret_taint_residual_smoke.sh

# LLVM dual-path: same refuse as emit-c on representative fail/pass
echo "=== llvm dual-path ==="
set +e
out=$("$OODAC_BIN" emit-llvm "$ROOT/fixtures/secret_sink_fail.oo" 2>&1)
rc=$?
set -e
if [[ $rc -ne 0 ]] && echo "$out" | grep -qE $'ERR\tsecret'; then
  echo "OK llvm secret_sink_fail refuses"
else
  echo "FAIL llvm secret_sink_fail" >&2
  fail=1
fi
set +e
out=$("$OODAC_BIN" emit-llvm "$ROOT/fixtures/secret_sink_pass.oo" 2>&1)
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  echo "OK llvm secret_sink_pass"
else
  echo "FAIL llvm secret_sink_pass rc=$rc" >&2
  fail=1
fi

if [[ $fail -ne 0 ]]; then
  echo "secret_product_floor_smoke: FAILED" >&2
  exit 1
fi
echo "secret_product_floor_smoke: PASSED"
