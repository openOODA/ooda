#!/usr/bin/env bash
# Caps product floor (alpha) — process-local seals + residual honesty
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODA="${OODA:-$ROOT/bin/ooda}"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
[[ -x "$OODA" ]] || { echo "ERR_NO_OODA" >&2; exit 1; }
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
run caps_matrix_smoke.sh
run alloc_cap_smoke.sh
run biometric_caps_residual_smoke.sh
run cap_ffi_residual_smoke.sh
if [[ $fail -ne 0 ]]; then
  echo "caps_product_floor_smoke: FAILED" >&2
  exit 1
fi
echo "caps_product_floor_smoke: PASSED"
