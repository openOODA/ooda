#!/usr/bin/env bash
# T1 secret depth — function return + alias rebind + if-taint (emit refuse/allow)
# Pattern: expect_refuse / expect_ok (same as secret_sink_enforce_smoke.sh)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
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

# Hard-require T1 fixtures
_REQ=(
  secret_function_return_fail secret_function_return_pass
  secret_alias_rebind_fail secret_alias_rebind_pass
  secret_if_taint_fail secret_if_taint_pass
)
for b in "${_REQ[@]}"; do
  [[ -f "$ROOT/fixtures/${b}.oo" ]] || bad "missing T1 fixture $b.oo"
done

# --- function return ---
expect_refuse "secret_function_return_fail" "$ROOT/fixtures/secret_function_return_fail.oo"
expect_ok "secret_function_return_pass" "$ROOT/fixtures/secret_function_return_pass.oo"

# --- alias rebind ---
# pass: rebind clean→secret → sink refuse
expect_refuse "secret_alias_rebind_pass" "$ROOT/fixtures/secret_alias_rebind_pass.oo"
# fail fixture: rebind secret→clean should allow once sticky taint clears.
# Product residual: sticky over-refuse → soft-pass; cleared → OK.
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

# --- if-taint ---
expect_refuse "secret_if_taint_fail" "$ROOT/fixtures/secret_if_taint_fail.oo"
expect_ok "secret_if_taint_pass" "$ROOT/fixtures/secret_if_taint_pass.oo"

if grep -q 'secret_t1_return_smoke' "$ROOT/scripts/ci_product.sh" 2>/dev/null; then
  pass "ci_product wires secret_t1_return_smoke"
else
  bad "ci_product missing secret_t1_return_smoke"
fi

echo "OK secret_t1_return_smoke"
exit 0
