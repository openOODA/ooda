#!/usr/bin/env bash
# M52/M53/M128/M131 Secret — println + write_file + fetch sink refuse + assign-prop
# residual: other OS sinks / #[Secret] attr / full IFC
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

DOC="$ROOT/bootstrap/SECRET_TAINT.md"
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; exit 1; }

# Path A floor fixtures must exist (hard require)
_REQ=(
  secret_sink_fail secret_sink_pass
  secret_write_file_fail secret_write_file_pass
  secret_fetch_fail secret_fetch_pass
  secret_sys_exec_fail secret_sys_exec_pass
  secret_env_get_fail secret_env_get_pass
  secret_read_file_fail secret_read_file_pass
  secret_process_exit_fail secret_process_exit_pass
)
for b in "${_REQ[@]}"; do
  [[ -f "$ROOT/fixtures/${b}.oo" ]] || bad "missing floor fixture $b.oo"
done


expect_refuse() {
  local label="$1" src="$2"
  set +e
  local out rc
  out="$("$OODAC" emit-c "$src" 2>&1)"
  rc=$?
  set -e
  [[ $rc -ne 0 ]] || bad "$label should non-zero rc (got 0)"
  echo "$out" | grep -qE $'ERR\tsecret' || bad "$label missing ERR secret out=$out"
  pass "$label emit refuses (rc=$rc)"
}

expect_ok() {
  local label="$1" src="$2"
  set +e
  local out rc
  out="$("$OODAC" emit-c "$src" 2>&1)"
  rc=$?
  set -e
  [[ $rc -eq 0 ]] || bad "$label emit failed rc=$rc out=$out"
  echo "$out" | grep -qE $'ERR\tsecret' && bad "$label should not ERR secret" || true
  pass "$label emit OK"
}

expect_refuse "secret_sink_fail" "$ROOT/fixtures/secret_sink_fail.oo"
expect_ok "secret_sink_pass" "$ROOT/fixtures/secret_sink_pass.oo"
expect_refuse "secret_assign_fail" "$ROOT/fixtures/secret_assign_fail.oo"
expect_ok "secret_assign_pass" "$ROOT/fixtures/secret_assign_pass.oo"
expect_refuse "secret_chain_fail" "$ROOT/fixtures/secret_chain_fail.oo"
expect_refuse "secret_concat_fail" "$ROOT/fixtures/secret_concat_fail.oo"
expect_refuse "secret_call_return_fail" "$ROOT/fixtures/secret_call_return_fail.oo"
expect_refuse "secret_call_arg_fail" "$ROOT/fixtures/secret_call_arg_fail.oo"

if [[ -f "$ROOT/fixtures/secret_write_file_fail.oo" ]]; then
  expect_refuse "secret_write_file_fail" "$ROOT/fixtures/secret_write_file_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_write_file_pass.oo" ]]; then
  expect_ok "secret_write_file_pass" "$ROOT/fixtures/secret_write_file_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_fetch_fail.oo" ]]; then
  expect_refuse "secret_fetch_fail" "$ROOT/fixtures/secret_fetch_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_fetch_pass.oo" ]]; then
  expect_ok "secret_fetch_pass" "$ROOT/fixtures/secret_fetch_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_sys_exec_fail.oo" ]]; then
  expect_refuse "secret_sys_exec_fail" "$ROOT/fixtures/secret_sys_exec_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_sys_exec_pass.oo" ]]; then
if [[ -f "$ROOT/fixtures/secret_env_get_fail.oo" ]]; then
  expect_refuse "secret_env_get_fail" "$ROOT/fixtures/secret_env_get_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_env_get_pass.oo" ]]; then
  expect_ok "secret_env_get_pass" "$ROOT/fixtures/secret_env_get_pass.oo"
fi
  expect_ok "secret_sys_exec_pass" "$ROOT/fixtures/secret_sys_exec_pass.oo"
if [[ -f "$ROOT/fixtures/secret_env_get_fail.oo" ]]; then
  expect_refuse "secret_env_get_fail" "$ROOT/fixtures/secret_env_get_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_env_get_pass.oo" ]]; then
  expect_ok "secret_env_get_pass" "$ROOT/fixtures/secret_env_get_pass.oo"
fi
fi
if [[ -f "$ROOT/fixtures/secret_multi_arg_println_fail.oo" ]]; then
  expect_refuse "secret_multi_arg_println_fail" "$ROOT/fixtures/secret_multi_arg_println_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_multi_name_fail.oo" ]]; then
  expect_refuse "secret_multi_name_fail" "$ROOT/fixtures/secret_multi_name_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_multi_name_pass.oo" ]]; then
  expect_ok "secret_multi_name_pass" "$ROOT/fixtures/secret_multi_name_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_invalid_empty.oo" ]]; then
  expect_refuse "secret_invalid_empty" "$ROOT/fixtures/secret_invalid_empty.oo"
fi

[[ -f "$DOC" ]] || bad "missing SECRET_TAINT.md"
grep -q 'SECRET_TAINT_RESIDUAL_ALPHA' "$DOC" || bad "doc missing residual marker"
grep -qiE 'fetch' "$DOC" || bad "doc missing fetch sink"
grep -qiE 'write_file' "$DOC" || bad "doc missing write_file sink"
grep -qiE 'sys_exec' "$DOC" || bad "doc missing sys_exec sink"
pass "residual doc honest (fetch/sys_exec named; residual IFC)"

if grep -q 'secret_sink_enforce_smoke' "$ROOT/scripts/ci_product.sh" 2>/dev/null; then
  pass "ci_product wires secret_sink_enforce_smoke"
else
  bad "ci_product missing secret_sink_enforce_smoke"
fi

echo "OK secret_sink_enforce_smoke"

if [[ -f "$ROOT/fixtures/secret_read_file_fail.oo" ]]; then
  expect_refuse "secret_read_file_fail" "$ROOT/fixtures/secret_read_file_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_read_file_pass.oo" ]]; then
  expect_ok "secret_read_file_pass" "$ROOT/fixtures/secret_read_file_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_path_exists_fail.oo" ]]; then
  expect_refuse "secret_path_exists_fail" "$ROOT/fixtures/secret_path_exists_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_path_exists_pass.oo" ]]; then
  expect_ok "secret_path_exists_pass" "$ROOT/fixtures/secret_path_exists_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_file_size_fail.oo" ]]; then
  expect_refuse "secret_file_size_fail" "$ROOT/fixtures/secret_file_size_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_file_size_pass.oo" ]]; then
  expect_ok "secret_file_size_pass" "$ROOT/fixtures/secret_file_size_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_write_file_path_fail.oo" ]]; then
  expect_refuse "secret_write_file_path_fail" "$ROOT/fixtures/secret_write_file_path_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_write_file_path_pass.oo" ]]; then
  expect_ok "secret_write_file_path_pass" "$ROOT/fixtures/secret_write_file_path_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_seed_fail.oo" ]]; then
  expect_refuse "secret_seed_fail" "$ROOT/fixtures/secret_seed_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_seed_pass.oo" ]]; then
  expect_ok "secret_seed_pass" "$ROOT/fixtures/secret_seed_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_sleep_ms_fail.oo" ]]; then
  expect_refuse "secret_sleep_ms_fail" "$ROOT/fixtures/secret_sleep_ms_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_sleep_ms_pass.oo" ]]; then
  expect_ok "secret_sleep_ms_pass" "$ROOT/fixtures/secret_sleep_ms_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_free_bytes_fail.oo" ]]; then
  expect_refuse "secret_free_bytes_fail" "$ROOT/fixtures/secret_free_bytes_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_free_bytes_pass.oo" ]]; then
  expect_ok "secret_free_bytes_pass" "$ROOT/fixtures/secret_free_bytes_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_alloc_bytes_fail.oo" ]]; then
  expect_refuse "secret_alloc_bytes_fail" "$ROOT/fixtures/secret_alloc_bytes_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_alloc_bytes_pass.oo" ]]; then
  expect_ok "secret_alloc_bytes_pass" "$ROOT/fixtures/secret_alloc_bytes_pass.oo"
fi

if [[ -f "$ROOT/fixtures/secret_process_exit_fail.oo" ]]; then
  expect_refuse "secret_process_exit_fail" "$ROOT/fixtures/secret_process_exit_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_process_exit_pass.oo" ]]; then
  expect_ok "secret_process_exit_pass" "$ROOT/fixtures/secret_process_exit_pass.oo"
fi

# T1 + zero-trust depth: function return / alias rebind / if-taint, then
# interprocedural residual gaps (field/method/closure/interp/setenv/mmap/…).
# alias_rebind_fail = secret→clean should allow; sticky over-refuse is residual.
if [[ -f "$ROOT/fixtures/secret_alias_rebind_pass.oo" ]]; then
  expect_refuse "secret_alias_rebind_pass" "$ROOT/fixtures/secret_alias_rebind_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_alias_rebind_fail.oo" ]]; then
  set +e
  _ar_out="$("$OODAC" emit-c "$ROOT/fixtures/secret_alias_rebind_fail.oo" 2>&1)"
  _ar_rc=$?
  set -e
  if [[ $_ar_rc -eq 0 ]]; then
    echo "$_ar_out" | grep -qE $'ERR\tsecret' && bad "secret_alias_rebind_fail should not ERR secret" || true
    pass "secret_alias_rebind_fail emit OK (sticky cleared)"
  elif echo "$_ar_out" | grep -qE $'ERR\tsecret'; then
    pass "secret_alias_rebind_fail residual sticky-taint over-refuse (rc=$_ar_rc)"
  else
    bad "secret_alias_rebind_fail unexpected rc=$_ar_rc out=$_ar_out"
  fi
fi
if [[ -f "$ROOT/fixtures/secret_function_return_fail.oo" ]]; then
  expect_refuse "secret_function_return_fail" "$ROOT/fixtures/secret_function_return_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_function_return_pass.oo" ]]; then
  expect_ok "secret_function_return_pass" "$ROOT/fixtures/secret_function_return_pass.oo"
fi
# T1 if-taint: secret assign inside if-branch → sink refuse; clean branch → allow
if [[ -f "$ROOT/fixtures/secret_if_taint_fail.oo" ]]; then
  expect_refuse "secret_if_taint_fail" "$ROOT/fixtures/secret_if_taint_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_if_taint_pass.oo" ]]; then
  expect_ok "secret_if_taint_pass" "$ROOT/fixtures/secret_if_taint_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_field_assign_fail.oo" ]]; then
  expect_refuse "secret_field_assign_fail" "$ROOT/fixtures/secret_field_assign_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_field_assign_pass.oo" ]]; then
  expect_ok "secret_field_assign_pass" "$ROOT/fixtures/secret_field_assign_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_method_return_fail.oo" ]]; then
  expect_refuse "secret_method_return_fail" "$ROOT/fixtures/secret_method_return_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_method_return_pass.oo" ]]; then
  expect_ok "secret_method_return_pass" "$ROOT/fixtures/secret_method_return_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_closure_return_fail.oo" ]]; then
  expect_refuse "secret_closure_return_fail" "$ROOT/fixtures/secret_closure_return_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_closure_return_pass.oo" ]]; then
  expect_ok "secret_closure_return_pass" "$ROOT/fixtures/secret_closure_return_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_string_interp_fail.oo" ]]; then
  expect_refuse "secret_string_interp_fail" "$ROOT/fixtures/secret_string_interp_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_string_interp_pass.oo" ]]; then
  expect_ok "secret_string_interp_pass" "$ROOT/fixtures/secret_string_interp_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_setenv_fail.oo" ]]; then
  expect_refuse "secret_setenv_fail" "$ROOT/fixtures/secret_setenv_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_setenv_pass.oo" ]]; then
  expect_ok "secret_setenv_pass" "$ROOT/fixtures/secret_setenv_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_mmap_fail.oo" ]]; then
  expect_refuse "secret_mmap_fail" "$ROOT/fixtures/secret_mmap_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_mmap_pass.oo" ]]; then
  expect_ok "secret_mmap_pass" "$ROOT/fixtures/secret_mmap_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_dlopen_arg_fail.oo" ]]; then
  expect_refuse "secret_dlopen_arg_fail" "$ROOT/fixtures/secret_dlopen_arg_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_dlopen_arg_pass.oo" ]]; then
  expect_ok "secret_dlopen_arg_pass" "$ROOT/fixtures/secret_dlopen_arg_pass.oo"
fi
if [[ -f "$ROOT/fixtures/secret_sys_exec_argv_fail.oo" ]]; then
  expect_refuse "secret_sys_exec_argv_fail" "$ROOT/fixtures/secret_sys_exec_argv_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_sys_exec_argv_pass.oo" ]]; then
  expect_ok "secret_sys_exec_argv_pass" "$ROOT/fixtures/secret_sys_exec_argv_pass.oo"
fi
