#!/usr/bin/env bash
# Contract fuzzer path A product floor (alpha) — PM 3.6
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODA="${OODA:-$ROOT/bin/ooda}"
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
run fuzz_int_depth_smoke.sh
run fuzz_bool_smoke.sh
run fuzz_string_smoke.sh
run fuzz_list_smoke.sh
run fuzz_multi_arg_smoke.sh
if [[ $fail -ne 0 ]]; then
  echo "fuzz_product_floor_smoke: FAILED" >&2
  exit 1
fi
echo "fuzz_product_floor_smoke: PASSED"
