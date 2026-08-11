#!/usr/bin/env bash
# job: local product rails (seed bootstrap + smokes + fixed_point)
# in:  SEED_OODAC (or bootstrap/seed/oodac) + gcc + bash
# out: exit 0 if product pure path green
#
# Residual honesty:
#  - Requires a prebuilt pure seed binary (cold start cannot invent a compiler from air).
#  - Does not prove a remote GitHub Actions matrix (see .github/workflows/product.yml); this is the local product rail.
#  - Does not uninstall system cargo if present — only refuses to *invoke* it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- product scripts must not shell out to host toolchain ---
# Allow comments/docs; flag bare cargo/rustc command lines only (anti-regression).
for s in "$ROOT/scripts/bootstrap_no_cargo.sh" "$ROOT/scripts/fixed_point.sh" \
         "$ROOT/scripts/release.sh"; do
  if grep -nE '^[[:space:]]*(cargo|rustc)([[:space:]]|$)' "$s" | grep -vE '^\s*#'; then
    bad "script invokes host toolchain: $s"
  else
    pass "product script clean: $(basename "$s")"
  fi
done

# Shadow host cargo/rustc if present (refuse accidental PATH use)
SHADOW="$TMPDIR/ci_product_shadow_$$"
mkdir -p "$SHADOW"
cat >"$SHADOW/cargo" <<'EOF'
#!/bin/sh
echo "ERR_SHADOW_CARGO" >&2
exit 99
EOF
cat >"$SHADOW/rustc" <<'EOF'
#!/bin/sh
echo "ERR_SHADOW_RUSTC" >&2
exit 99
EOF
chmod +x "$SHADOW/cargo" "$SHADOW/rustc"
export PATH="$SHADOW:$PATH"

if cargo version >/dev/null 2>&1; then
  bad "host toolchain shadow did not intercept"
else
  pass "host toolchain shadowed"
fi

# Product tree purity
RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' | wc -l)
echo "RS_COUNT=$RS"
[[ "$RS" -eq 0 ]] && pass "product tree purity: RS=0" || bad "product tree purity: RS=$RS"
[[ ! -f "$ROOT/Cargo.toml" ]] && pass "product tree purity: no Cargo.toml" || bad "Cargo.toml present"
[[ ! -d "$ROOT/src" ]] && pass "product tree purity: no src/" || bad "src/ present"

# Product path — default cold seed (not tree oodac; bootstrap rm's STAGE1).
_SEED_DEFAULT="$ROOT/bootstrap/seed/oodac"
if [[ ! -x "$_SEED_DEFAULT" ]]; then _SEED_DEFAULT="$ROOT/oodac/oodac"; fi
if ! SEED_OODAC="${SEED_OODAC:-$_SEED_DEFAULT}" "$ROOT/scripts/bootstrap_no_cargo.sh" \
  >"$TMPDIR/ci_boot.out" 2>"$TMPDIR/ci_boot.err"; then
  bad "bootstrap_no_cargo"
  cat "$TMPDIR/ci_boot.err" | tail -20
else
  pass "bootstrap_no_cargo under cargo-shadow PATH"
fi

export OODA="$ROOT/bin/ooda"
export OODAC_BIN="$ROOT/oodac/oodac"

for rail in \
  product_pure_dispatch_smoke.sh \
  verify_pure_smoke.sh \
  p3_no_cargo_smoke.sh \
  chs_parity.sh \
  beta_cli_smoke.sh \
  c_emit_smoke.sh \
  wasm_emit_smoke.sh \
  wasm_execute_smoke.sh \
  llvm_token_align_smoke.sh \
  llvm_execute_smoke.sh \
  llvm_prod_parity_smoke.sh \
  llvm_fail_closed_smoke.sh \
  bc_vm_smoke.sh \
  problem_hunt_smoke.sh \
  caps_matrix_smoke.sh \
  libfloor_process_smoke.sh \
  libfloor_net_smoke.sh \
  alloc_cap_smoke.sh \
  list_quota_smoke.sh \
  import_load_smoke.sh \
  contracts_native_smoke.sh \
  contracts_multi_clause_smoke.sh \
  json_errors_smoke.sh \
  ai_native_product_floor_smoke.sh \
  hitl_product_floor_smoke.sh \
  cap_ffi_runtime_smoke.sh \
  libfloor_mutex_thread_smoke.sh \
  libfloor_thread_gpu_smoke.sh \
  ecap_autofix_smoke.sh \
  secret_eprintln_smoke.sh \
  contracts_and_smoke.sh \
  etc_autofix_smoke.sh \
  m162_residual_deepen_smoke.sh \
  thread_join_smoke.sh \
  channel_path_a_smoke.sh \
  byte_str_path_a_smoke.sh \
  bytes_buffer_smoke.sh \
  tls_path_a_smoke.sh \
  outline_reflect_smoke.sh \
  patch_smoke.sh \
  std_smoke.sh \
  shell_safety_smoke.sh \
  arc_smoke.sh \
  fuzz_int_depth_smoke.sh \
  fuzz_bool_smoke.sh \
  fuzz_string_smoke.sh \
  fuzz_list_smoke.sh \
  fuzz_multi_arg_smoke.sh \
  run_engine_parity_smoke.sh \
  residual_honesty_smoke.sh \
  max_cycles_enforce_smoke.sh \
  max_cycles_for_enforce_smoke.sh \
  max_cycles_shared_smoke.sh \
  max_cycles_recursion_smoke.sh \
  max_cycles_multi_digit_smoke.sh \
  max_cycles_residual_smoke.sh \
  secret_sink_enforce_smoke.sh \
  secret_taint_residual_smoke.sh \
  hitl_residual_smoke.sh \
  type_state_residual_smoke.sh \
  toolchains_adv_residual_smoke.sh \
  temporal_mem_residual_smoke.sh \
  telepathic_ast_residual_smoke.sh \
  shadow_state_residual_smoke.sh \
  native_lsp_residual_smoke.sh \
  meta_vs_det_residual_smoke.sh \
  metamorphic_residual_smoke.sh \
  lto_xlang_residual_smoke.sh \
  hot_reload_residual_smoke.sh \
  holographic_residual_smoke.sh \
  hivemind_residual_smoke.sh \
  gpu_npu_residual_smoke.sh \
  ffi_gen_residual_smoke.sh \
  dod_layout_residual_smoke.sh \
  concurrency_residual_smoke.sh \
  callgraph_crypto_residual_smoke.sh \
  bare_metal_residual_smoke.sh \
  byte_str_residual_smoke.sh \
  ast_macros_residual_smoke.sh \
  cap_ffi_product_floor_smoke.sh \
  residual_path_a_floor_smoke.sh \
  cap_ffi_residual_smoke.sh \
  biometric_caps_residual_smoke.sh \
  arc_temporal_tension_residual_smoke.sh \
  ooda_speed_residual_smoke.sh \
  qa_matrix_residual_smoke.sh \
  spec_depth_residual_smoke.sh \
  ebnf_align_residual_smoke.sh \
  playground_residual_smoke.sh \
  pkg_ecosystem_residual_smoke.sh \
  std_split_residual_smoke.sh \
  seed_pure_multi_smoke.sh \
  multi_target_residual_smoke.sh \
  bc_vm_depth_residual_smoke.sh \
  contracts_complex_residual_smoke.sh \
  ast_autofix_residual_smoke.sh \
  llvm_smoke_residual_smoke.sh \
  wasm_smoke_residual_smoke.sh \
  residual_packs_index_smoke.sh \
  pure_build_fp_smoke.sh
do
  if [[ ! -x "$ROOT/scripts/$rail" ]]; then
    if [[ -f "$ROOT/scripts/$rail" ]]; then
      chmod +x "$ROOT/scripts/$rail" 2>/dev/null || true
    fi
  fi
  if [[ ! -x "$ROOT/scripts/$rail" ]]; then
    bad "missing rail $rail"
    continue
  fi
  if ! "$ROOT/scripts/$rail" >"$TMPDIR/ci_$rail.out" 2>"$TMPDIR/ci_$rail.err"; then
    bad "$rail"
    tail -15 "$TMPDIR/ci_$rail.err" || tail -15 "$TMPDIR/ci_$rail.out" || true
  else
    pass "$rail"
  fi
done

if ! "$ROOT/scripts/fixed_point.sh" >"$TMPDIR/ci_fp.out" 2>"$TMPDIR/ci_fp.err"; then
  bad "fixed_point"
  tail -20 "$TMPDIR/ci_fp.out"
else
  pass "fixed_point pure seed"
  if grep -q 'OK_HOST' "$TMPDIR/ci_fp.out" 2>/dev/null; then
    bad "OK_HOST in fixed_point log"
  else
    pass "no OK_HOST in fixed_point log"
  fi
fi

# Ensure host-toolchain shadow was never hit
if grep -rq 'cargo invoked on no-Rust\|ERR_SHADOW_CARGO\|ERR_SHADOW_RUSTC' "$TMPDIR"/ci_*.err "$TMPDIR"/ci_*.out 2>/dev/null; then
  bad "a rail tried to invoke host toolchain"
else
  pass "no rail invoked host toolchain"
fi

if [[ $fail -ne 0 ]]; then
  echo "ci_product: FAILED" >&2
  exit 1
fi
echo "ci_product: PASSED"
echo "residual: prebuilt SEED_OODAC required (bootstrap/seed, tree oodac, or pin release asset)"
echo "remote: .github/workflows/product.yml"
exit 0
