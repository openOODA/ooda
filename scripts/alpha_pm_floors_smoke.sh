#!/usr/bin/env bash
# Run all product-floor umbrellas + key partial-floor rails for PM alpha pass
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODA="${OODA:-$ROOT/bin/ooda}"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
run(){ echo "=== $1 ==="; bash "$ROOT/scripts/$1" || { echo "FAIL $1" >&2; fail=1; }; }
# already done floors
for s in \
  caps_product_floor_smoke.sh \
  cap_ffi_product_floor_smoke.sh \
  residual_path_a_floor_smoke.sh \
  time_entropy_product_floor_smoke.sh \
  memory_quota_product_floor_smoke.sh \
  max_cycles_product_floor_smoke.sh \
  secret_product_floor_smoke.sh \
  fuzz_product_floor_smoke.sh \
  contracts_native_smoke.sh \
  contracts_multi_clause_smoke.sh \
  contracts_complex_residual_smoke.sh \
  json_errors_smoke.sh \
  ai_native_product_floor_smoke.sh \
  hitl_product_floor_smoke.sh \
  cap_ffi_runtime_smoke.sh \
  ecap_autofix_smoke.sh \
  secret_eprintln_smoke.sh \
  contracts_and_smoke.sh \
  etc_autofix_smoke.sh \
  patch_smoke.sh \
  ast_autofix_residual_smoke.sh \
  arc_smoke.sh \
  arc_temporal_tension_residual_smoke.sh \
  pure_build_fp_smoke.sh \
  llvm_prod_parity_smoke.sh \
  llvm_execute_smoke.sh \
  llvm_fail_closed_smoke.sh \
  wasm_emit_smoke.sh \
  wasm_execute_smoke.sh \
  wasm_smoke_residual_smoke.sh \
  ooda_speed_residual_smoke.sh \
  ebnf_align_residual_smoke.sh \
  residual_packs_index_smoke.sh \
  residual_honesty_smoke.sh
 do
  [[ -f "$ROOT/scripts/$s" ]] || { echo "SKIP missing $s"; continue; }
  run "$s"
done
[[ $fail -eq 0 ]] || { echo "alpha_pm_floors_smoke: FAILED" >&2; exit 1; }
echo "alpha_pm_floors_smoke: PASSED"
