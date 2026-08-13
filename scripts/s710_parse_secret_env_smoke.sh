#!/usr/bin/env bash
# job: 7.10 c_parse_secret_env path-A smoke
# in:  OODAC_BIN (default oodac/oodac) + ephemeral TMPDIR fixtures
# out: exit 0 only if fail-closed + multi-name + empty-name rails hold
#
# Rails:
#   FAIL-closed — // SECRET: tok then println(tok)
#   PASS        — two SECRET names both refuse (println(a) and println(b))
#   FAIL-closed — // SECRET:   (empty) invalid name
#
# Residual: full IFC / #[Secret] / interp ${} / char-by-char name concat hygiene.
# Does not rewrite Domain Expert product logic (c_emit_secret.oo).
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

FIX="$TMPDIR/s710_parse_secret_env_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT

# --- fixtures (all under TMPDIR; never pollute repo root) ---

# FAIL-CLOSED: SECRET ident at println must refuse
cat >"$FIX/println_ident.oo" <<'EOF'
// 7.10 path-A: bare SECRET IDENT at println must refuse
// SECRET: tok
pub fn main() {
    let tok = "s3cret";
    println(tok);
}
EOF

# PASS: two SECRET names — first name refuse
cat >"$FIX/two_names_a.oo" <<'EOF'
// 7.10 path-A: two SECRET tags; sink on first name must refuse
// SECRET: a
// SECRET: b
pub fn main() {
    let a = "x";
    let b = "y";
    println(a);
}
EOF

# PASS: two SECRET names — second name refuse
cat >"$FIX/two_names_b.oo" <<'EOF'
// 7.10 path-A: two SECRET tags; sink on second name must refuse
// SECRET: a
// SECRET: b
pub fn main() {
    let a = "x";
    let b = "y";
    println(b);
}
EOF

# FAIL-CLOSED: empty name after // SECRET:   is invalid
cat >"$FIX/empty_name.oo" <<'EOF'
// 7.10 path-A: empty SECRET name (spaces only) must refuse invalid name
// SECRET:   
pub fn main() {
    println(1);
}
EOF

# --- helpers ---

expect_refuse() {
  local src="$1"
  local label="$2"
  local needle="${3:-}"
  local out="$FIX/${label}.out"
  set +e
  "$OODAC" emit-c "$src" >"$out" 2>&1
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    bad "fail-closed accepted (want ERR secret): $label"
    head -8 "$out" >&2 || true
    return 1
  fi
  if ! grep -qE $'ERR\tsecret' "$out"; then
    bad "fail-closed missing ERR secret: $label (got: $(head -2 "$out"))"
    return 1
  fi
  if [[ -n "$needle" ]] && ! grep -qF "$needle" "$out"; then
    bad "fail-closed missing name '$needle': $label (got: $(head -2 "$out"))"
    return 1
  fi
  pass "fail-closed: $label"
  return 0
}

# --- rails ---

expect_refuse "$FIX/println_ident.oo" "secret_tok_println_ident" "tok" || true
expect_refuse "$FIX/two_names_a.oo" "secret_two_names_refuse_a" "SECRET a" || true
expect_refuse "$FIX/two_names_b.oo" "secret_two_names_refuse_b" "SECRET b" || true
expect_refuse "$FIX/empty_name.oo" "secret_empty_invalid_name" "invalid name" || true

if [[ $fail -ne 0 ]]; then
  echo "s710_parse_secret_env_smoke: FAILED" >&2
  exit 1
fi
echo "s710_parse_secret_env_smoke: ALL OK"
echo "RESIDUAL full IFC / #[Secret] attr / string-interp \${} deep taint — not claimed closed"
echo "RESIDUAL 7.10 concat hygiene is product (c_parse_secret_env name = name + c); this smoke proves parse/refuse rails only"
exit 0
