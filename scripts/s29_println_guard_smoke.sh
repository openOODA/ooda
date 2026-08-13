#!/usr/bin/env bash
# job: 2.9 println walker → c_secret_emit_guard path-A smoke
# in:  OODAC_BIN (default oodac/oodac) + ephemeral TMPDIR fixtures
# out: exit 0 only if fail-closed + no-FP rails hold
#
# Rails (emit-c AND check — dual-path):
#   FAIL-closed — // SECRET: tok then println(tok) bare IDENT
#   PASS no-FP  — println("tok") string literal when tok is SECRET name
#   FAIL-closed — // SECRET: tok then eprintln(tok) if product names eprintln
#   FAIL-closed — // SECRET: tok then println((1), tok) (H-later: first RPAREN
#                 is grouping, not the sink closer)
#   FAIL-closed — // SECRET: tok then println(("x") + tok) (H-same: tok after
#                 inner grouping RPAREN in the same arg)
#   PASS no-FP  — println((1), 2) with unused SECRET tok (grouping, no IDENT)
#
# Residual: full IFC / #[Secret] / interp ${} / walker→guard unify is Domain
#           Expert product. This smoke proves refuse/no-FP, not the helper.
# Does not rewrite Domain Expert product (c_emit_secret*.oo / c_emit_print.oo).
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

FIX="$TMPDIR/s29_println_guard_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT

PRODUCT_WALK="$ROOT/oodac/c_emit_secret.oo"
PRODUCT_GUARD="$ROOT/oodac/c_emit_secret_guard.oo"
PRODUCT_ALIAS="$ROOT/oodac/c_emit_secret_alias.oo"

# --- fixtures (all under TMPDIR; never pollute repo root) ---

# FAIL-CLOSED: SECRET ident at println must refuse
cat >"$FIX/println_ident.oo" <<'EOF'
// 2.9 path-A: bare SECRET IDENT at println must refuse
// SECRET: tok
pub fn main() {
    let tok = "s3cret";
    println(tok);
}
EOF

# NO-FP: string literal spelling of SECRET name must not refuse
cat >"$FIX/println_string_lit.oo" <<'EOF'
// 2.9 path-A: println("tok") is a string literal, not IDENT — no false positive
// SECRET: tok
pub fn main() {
    println("tok");
}
EOF

# FAIL-CLOSED: SECRET ident at eprintln (product sibling sink)
cat >"$FIX/eprintln_ident.oo" <<'EOF'
// 2.9 path-A: bare SECRET IDENT at eprintln must refuse (if product)
// SECRET: tok
pub fn main() {
    let tok = "s3cret";
    eprintln(tok);
}
EOF

# FAIL-CLOSED: grouped first arg, SECRET later IDENT (H-later)
cat >"$FIX/println_grouped_later.oo" <<'EOF'
// 2.9 path-A: println((1), tok) must refuse (not stop at first RPAREN)
// SECRET: tok
pub fn main() {
    let tok = "s3cret";
    println((1), tok);
}
EOF

# FAIL-CLOSED: grouping then + SECRET in the same arg (H-same)
cat >"$FIX/println_groupplus.oo" <<'EOF'
// 2.9 path-A: println(("x") + tok) must refuse (tok after inner RPAREN)
// SECRET: tok
pub fn main() {
    let tok = "s3cret";
    println(("x") + tok);
}
EOF

# NO-FP: grouped public args while a SECRET name exists unused
cat >"$FIX/println_grouped_public.oo" <<'EOF'
// 2.9 path-A: println((1), 2) is public grouping — no false positive
// SECRET: tok
pub fn main() {
    println((1), 2);
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
  if grep -qE $'ERR\tsecret' "$out"; then
    bad "no-FP $cmd has ERR secret: $label"
    return 1
  fi
  if [[ $rc -ne 0 ]]; then
    bad "no-FP $cmd refused (want OK): $label rc=$rc"
    head -8 "$out" >&2 || true
    return 1
  fi
  pass "no-FP $cmd: $label"
  return 0
}

expect_ok() {
  local src="$1"
  local label="$2"
  expect_ok_cmd emit-c "$src" "$label" || return 1
  expect_ok_cmd check "$src" "$label" || return 1
}

product_has_eprintln() {
  local f
  for f in "$PRODUCT_WALK" "$PRODUCT_GUARD" "$PRODUCT_ALIAS"; do
    [[ -f "$f" ]] || continue
    if grep -qE 'sid == "eprintln"|sink == "eprintln"|name == "eprintln"' "$f"; then
      return 0
    fi
  done
  return 1
}

# --- rails ---

expect_refuse "$FIX/println_ident.oo" "secret_tok_println_ident" || true
expect_ok "$FIX/println_string_lit.oo" "secret_tok_println_string_lit" || true
expect_refuse "$FIX/println_grouped_later.oo" "secret_tok_println_grouped_later" || true
expect_refuse "$FIX/println_groupplus.oo" "secret_tok_println_groupplus" || true
expect_ok "$FIX/println_grouped_public.oo" "secret_tok_println_grouped_public" || true

if product_has_eprintln; then
  expect_refuse "$FIX/eprintln_ident.oo" "secret_tok_eprintln_ident" || true
else
  pass "eprintln rail skipped (not in product SECRETSINK)"
fi

if [[ $fail -ne 0 ]]; then
  echo "s29_println_guard_smoke: FAILED" >&2
  exit 1
fi
echo "s29_println_guard_smoke: ALL OK"
echo "RESIDUAL full IFC / #[Secret] attr / string-interp \${} deep taint — not claimed closed"
echo "RESIDUAL 2.9 walker→c_secret_emit_guard unify is Domain Expert product; this smoke proves refuse/no-FP"
exit 0
