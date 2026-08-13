#!/usr/bin/env bash
# job: 6.1 env_set SECRETSINK path-A smoke
# in:  OODAC_BIN (default oodac/oodac) + ephemeral TMPDIR fixtures
# out: exit 0 only if fail-closed + no-FP rails hold
#
# Rails (emit-c AND check — dual-path):
#   FAIL-closed — // SECRET: tok then env_set(..., tok) bare IDENT
#   FAIL-closed — // SECRET: tok then setenv/unsetenv(..., tok) if product names them
#   PASS no-FP  — env_set(..., "tok") string literal when tok is SECRET name
#
# Residual: full IFC / #[Secret] / interp ${} / env_set product lower.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

FIX="$TMPDIR/s61_env_set_secretsink_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT

PRODUCT_GUARD="$ROOT/oodac/c_emit_secret_guard.oo"
PRODUCT_ALIAS="$ROOT/oodac/c_emit_secret_alias.oo"

# --- fixtures (all under TMPDIR; never pollute repo root) ---

# FAIL-CLOSED: SECRET ident as env_set value must refuse
cat >"$FIX/env_set_ident.oo" <<'EOF'
// 6.1 path-A: bare SECRET IDENT at env_set must refuse
// SECRET: tok
pub fn main(envc: &EnvCap) {
    let tok = "s3cret";
    let r = env_set(envc, "KEY", tok);
    println(1);
}
EOF

# FAIL-CLOSED: SECRET ident as setenv value (product sibling sink)
cat >"$FIX/setenv_ident.oo" <<'EOF'
// 6.1 path-A: bare SECRET IDENT at setenv must refuse (if product)
// SECRET: tok
pub fn main(envc: &EnvCap) {
    let tok = "s3cret";
    let r = setenv(envc, "KEY", tok);
    println(1);
}
EOF

# FAIL-CLOSED: SECRET ident as unsetenv name (product sibling sink)
cat >"$FIX/unsetenv_ident.oo" <<'EOF'
// 6.1 path-A: bare SECRET IDENT at unsetenv must refuse (if product)
// SECRET: tok
pub fn main(envc: &EnvCap) {
    let tok = "KEY";
    let r = unsetenv(envc, tok);
    println(1);
}
EOF

# NO-FP: string literal spelling of SECRET name must not refuse
cat >"$FIX/env_set_string_lit.oo" <<'EOF'
// 6.1 path-A: env_set(..., "tok") is a string literal, not IDENT — no false positive
// SECRET: tok
pub fn main(envc: &EnvCap) {
    let r = env_set(envc, "KEY", "tok");
    println(1);
}
EOF

# --- helpers ---

expect_refuse_cmd() {
  local cmd="$1"
  local src="$2"
  local label="$3"
  local out="$FIX/${label}.${cmd}.out"
  set +e
  "$OODAC" "$cmd" "$src" >"$out" 2>&1
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    bad "fail-closed $cmd accepted (want ERR secret): $label"
    head -8 "$out" >&2 || true
    return 1
  fi
  if ! grep -qE $'ERR\tsecret' "$out"; then
    bad "fail-closed $cmd missing ERR secret: $label (got: $(tr -d '\n' <"$out" | head -c 160))"
    return 1
  fi
  pass "fail-closed $cmd: $label"
  return 0
}

expect_refuse() {
  local src="$1"
  local label="$2"
  expect_refuse_cmd emit-c "$src" "$label" || return 1
  expect_refuse_cmd check "$src" "$label" || return 1
}

expect_ok_cmd() {
  local cmd="$1"
  local src="$2"
  local label="$3"
  local out="$FIX/${label}.${cmd}.out"
  set +e
  "$OODAC" "$cmd" "$src" >"$out" 2>&1
  local rc=$?
  set -e
  # no-FP = string literal must not trip SECRET refuse.
  # env_set may still be unlowered (unknown free call / type residual).
  if grep -qE $'ERR\tsecret' "$out"; then
    bad "no-FP $cmd has ERR secret: $label"
    return 1
  fi
  if [[ $rc -eq 0 ]]; then
    pass "no-FP $cmd: $label"
    return 0
  fi
  if grep -qE 'unknown free call.*env_set|undefined variable .env_set.' "$out"; then
    pass "no-FP $cmd: $label (no ERR secret; env_set product-lower residual)"
    return 0
  fi
  bad "no-FP $cmd refused (want OK or unknown-call residual): $label rc=$rc"
  head -8 "$out" >&2 || true
  return 1
}

expect_ok() {
  local src="$1"
  local label="$2"
  expect_ok_cmd emit-c "$src" "$label" || return 1
  expect_ok_cmd check "$src" "$label" || return 1
}

product_has_setenv() {
  [[ -f "$PRODUCT_GUARD" ]] || return 1
  if grep -qE 'sink == "setenv"|sid == "setenv"' "$PRODUCT_GUARD"; then
    return 0
  fi
  if [[ -f "$PRODUCT_ALIAS" ]] && grep -qE 'name == "setenv"' "$PRODUCT_ALIAS"; then
    return 0
  fi
  return 1
}

# --- rails ---

expect_refuse "$FIX/env_set_ident.oo" "secret_tok_env_set_ident" || true

if product_has_setenv; then
  expect_refuse "$FIX/setenv_ident.oo" "secret_tok_setenv_ident" || true
  expect_refuse "$FIX/unsetenv_ident.oo" "secret_tok_unsetenv_ident" || true
else
  pass "setenv/unsetenv rail skipped (not in product SECRETSINK)"
fi

expect_ok "$FIX/env_set_string_lit.oo" "secret_tok_env_set_string_lit" || true

if [[ $fail -ne 0 ]]; then
  echo "s61_env_set_secretsink_smoke: FAILED" >&2
  exit 1
fi
echo "s61_env_set_secretsink_smoke: ALL OK"
echo "RESIDUAL full IFC / #[Secret] attr / string-interp \${} deep taint — not claimed closed"
echo "RESIDUAL env_set product lower (link-time) is CAPS residual; this smoke proves emit-c+check refuse/no-FP"
exit 0
