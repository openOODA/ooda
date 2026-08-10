#!/usr/bin/env bash
# M52/M53 Secret — Backend-C println bare-IDENT refuse + direct IDENT assign-prop
# residual: NetCap/fetch / #[Secret] attr / full IFC (interproc+concat+write_file In)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

DOC="$ROOT/bootstrap/SECRET_TAINT.md"
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

expect_refuse "secret_sink_fail" "$ROOT/fixtures/secret_sink_fail.oo"
expect_ok "secret_sink_pass" "$ROOT/fixtures/secret_sink_pass.oo"
expect_refuse "secret_assign_fail" "$ROOT/fixtures/secret_assign_fail.oo"
expect_ok "secret_assign_pass" "$ROOT/fixtures/secret_assign_pass.oo"
expect_refuse "secret_chain_fail" "$ROOT/fixtures/secret_chain_fail.oo"
expect_refuse "secret_concat_fail" "$ROOT/fixtures/secret_concat_fail.oo"
expect_refuse "secret_call_return_fail" "$ROOT/fixtures/secret_call_return_fail.oo"
expect_refuse "secret_call_arg_fail" "$ROOT/fixtures/secret_call_arg_fail.oo"
# M128 write_file non-println sink
if [[ -f "$ROOT/fixtures/secret_write_file_fail.oo" ]]; then
  expect_refuse "secret_write_file_fail" "$ROOT/fixtures/secret_write_file_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_write_file_pass.oo" ]]; then
  expect_ok "secret_write_file_pass" "$ROOT/fixtures/secret_write_file_pass.oo"
fi
# M65 empty SECRET name fail-closed
if [[ -f "$ROOT/fixtures/secret_invalid_empty.oo" ]]; then
  
if [[ -f "$ROOT/fixtures/secret_multi_arg_println_fail.oo" ]]; then
  expect_refuse "secret_multi_arg_println_fail" "$ROOT/fixtures/secret_multi_arg_println_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_multi_name_fail.oo" ]]; then
  expect_refuse "secret_multi_name_fail" "$ROOT/fixtures/secret_multi_name_fail.oo"
fi
if [[ -f "$ROOT/fixtures/secret_multi_name_pass.oo" ]]; then
  expect_ok "secret_multi_name_pass" "$ROOT/fixtures/secret_multi_name_pass.oo"
fi

expect_refuse "secret_invalid_empty" "$ROOT/fixtures/secret_invalid_empty.oo"
fi


[[ -f "$DOC" ]] || bad "missing SECRET_TAINT.md"
grep -q 'SECRET_TAINT_RESIDUAL_ALPHA' "$DOC" || bad "doc missing residual marker"
grep -qiE 'NetCap|netcap' "$DOC" || bad "doc missing NetCap residual"
grep -qiE 'write_file' "$DOC" || bad "doc missing write_file sink"
pass "residual doc still honest (NetCap residual; write_file named)"

if grep -q 'secret_sink_enforce_smoke' "$ROOT/scripts/ci_product.sh" 2>/dev/null; then
  pass "ci_product wires secret_sink_enforce_smoke"
else
  bad "ci_product missing secret_sink_enforce_smoke"
fi


# M55 check dual-path (path A names; assign-prop still emit-only)
set +e
out_ck="$("$OODAC" check "$ROOT/fixtures/secret_sink_fail.oo" 2>&1)"
rc_ck=$?
set -e
[[ $rc_ck -ne 0 ]] || bad "check secret_sink_fail should non-zero"
echo "$out_ck" | grep -qE $'ERR\tsecret' || bad "check secret_sink_fail missing ERR secret out=$out_ck"
pass "check dual-path refuses secret_sink_fail"

set +e
out_ckp="$("$OODAC" check "$ROOT/fixtures/secret_sink_pass.oo" 2>&1)"
rc_ckp=$?
set -e
[[ $rc_ckp -eq 0 ]] || bad "check secret_sink_pass failed out=$out_ckp"
echo "$out_ckp" | grep -qE $'ERR\tsecret' && bad "check pass should not ERR secret" || true
pass "check dual-path allows secret_sink_pass"

# M60: check dual-path path B assign-prop
set +e
out_cka="$("$OODAC" check "$ROOT/fixtures/secret_assign_fail.oo" 2>&1)"
rc_cka=$?
set -e
[[ $rc_cka -ne 0 ]] || bad "check secret_assign_fail should non-zero"
echo "$out_cka" | grep -qE $'ERR\tsecret' || bad "check assign_fail missing ERR secret out=$out_cka"
pass "check dual-path refuses secret_assign_fail"

set +e
out_ckc="$("$OODAC" check "$ROOT/fixtures/secret_chain_fail.oo" 2>&1)"
rc_ckc=$?
set -e
[[ $rc_ckc -ne 0 ]] || bad "check secret_chain_fail should non-zero"
echo "$out_ckc" | grep -qE $'ERR\tsecret' || bad "check chain_fail missing ERR secret out=$out_ckc"
pass "check dual-path refuses secret_chain_fail"

echo "secret_sink_enforce_smoke: PASSED"
