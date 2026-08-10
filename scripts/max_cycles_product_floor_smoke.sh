#!/usr/bin/env bash
# MaxCycles product floor (alpha) — all path-A rails must pass
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
OODA="${OODA_BIN:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export OODA OODAC_BIN="$OODAC"
[[ -x "$OODA" ]] || { echo "ERR_NO_OODA" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
run() {
  local s="$1"
  echo "=== $s ==="
  if bash "$ROOT/scripts/$s"; then
    echo "OK $s"
  else
    echo "FAIL $s" >&2
    fail=1
  fi
}
run max_cycles_enforce_smoke.sh
run max_cycles_for_enforce_smoke.sh
run max_cycles_shared_smoke.sh
run max_cycles_recursion_smoke.sh
run max_cycles_multi_digit_smoke.sh
run max_cycles_residual_smoke.sh
if [[ $fail -ne 0 ]]; then
  echo "max_cycles_product_floor_smoke: FAILED" >&2
  exit 1
fi
echo "max_cycles_product_floor_smoke: PASSED"
