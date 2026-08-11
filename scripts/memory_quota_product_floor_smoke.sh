#!/usr/bin/env bash
# PM 3.3 Memory quotas product floor (alpha)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODA="${OODA:-$ROOT/bin/ooda}"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
[[ -x "$OODA" ]] || { echo "ERR_NO_OODA" >&2; exit 1; }
fail=0
run(){ echo "=== $1 ==="; bash "$ROOT/scripts/$1" || { echo "FAIL $1" >&2; fail=1; }; }
run alloc_cap_smoke.sh
run list_quota_smoke.sh
run malloc_path_a_smoke.sh
[[ $fail -eq 0 ]] || { echo "memory_quota_product_floor_smoke: FAILED" >&2; exit 1; }
echo "memory_quota_product_floor_smoke: PASSED"
