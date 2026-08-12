#!/usr/bin/env bash
# T2 secret scan depth — field / method / args / comment-skip (emit refuse/allow)
# Pattern: expect_refuse / expect_ok (same as secret_t1_return_smoke.sh)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; exit 1; }

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

# Hard-require T2 fixtures (field / method / args / comment-skip)
_REQ=(
  secret_field_assign_fail secret_field_assign_pass
  secret_method_return_fail secret_method_return_pass
  secret_call_arg_fail secret_multi_arg_println_fail
  secret_multi_name_fail secret_multi_name_pass
  secret_midline_safe
)
for b in "${_REQ[@]}"; do
  [[ -f "$ROOT/fixtures/${b}.oo" ]] || bad "missing T2 fixture $b.oo"
done

# --- field assign taint ---
expect_refuse "secret_field_assign_fail" "$ROOT/fixtures/secret_field_assign_fail.oo"
expect_ok "secret_field_assign_pass" "$ROOT/fixtures/secret_field_assign_pass.oo"

# --- method return taint ---
expect_refuse "secret_method_return_fail" "$ROOT/fixtures/secret_method_return_fail.oo"
expect_ok "secret_method_return_pass" "$ROOT/fixtures/secret_method_return_pass.oo"

# --- args scan (call arg prop + multi-arg sink + multi SECRET names) ---
expect_refuse "secret_call_arg_fail" "$ROOT/fixtures/secret_call_arg_fail.oo"
expect_refuse "secret_multi_arg_println_fail" "$ROOT/fixtures/secret_multi_arg_println_fail.oo"
expect_refuse "secret_multi_name_fail" "$ROOT/fixtures/secret_multi_name_fail.oo"
expect_ok "secret_multi_name_pass" "$ROOT/fixtures/secret_multi_name_pass.oo"

# optional FFI / argv payload args (present on product floor)
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

# --- comment-skip: mid-line // SECRET: is not a directive tag ---
expect_ok "secret_midline_safe" "$ROOT/fixtures/secret_midline_safe.oo"
# structural: fixture must mention SECRET mid-comment without line-start directive
grep -qE '//.*// SECRET:' "$ROOT/fixtures/secret_midline_safe.oo" \
  || grep -qE 'Mention.*SECRET' "$ROOT/fixtures/secret_midline_safe.oo" \
  || bad "secret_midline_safe missing mid-line SECRET mention"
if grep -qE '^[[:space:]]*// SECRET:[[:space:]]*[[:alnum:]_]+' "$ROOT/fixtures/secret_midline_safe.oo"; then
  bad "secret_midline_safe must not have line-start // SECRET: directive"
fi
pass "comment-skip fixture shape (no line-start SECRET directive)"

if grep -q 'secret_t2_depth_smoke' "$ROOT/scripts/ci_product.sh" 2>/dev/null; then
  pass "ci_product wires secret_t2_depth_smoke"
else
  bad "ci_product missing secret_t2_depth_smoke"
fi

echo "OK secret_t2_depth_smoke"
exit 0
