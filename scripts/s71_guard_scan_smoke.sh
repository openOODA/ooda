#!/usr/bin/env bash
# job: 7.1 c_guard_scan_idents path-A smoke
# in:  OODAC_BIN (default oodac/oodac) + ephemeral TMPDIR fixtures
# out: exit 0 only if fail-closed + no-FP rails hold
#
# Rails:
#   PASS fail-closed — // SECRET: tok then println(tok)
#   PASS fail-closed — // SECRET: tok then write_file payload tok
#   PASS no-FP       — println("tok") string literal when tok is SECRET name
#
# Residual: full IFC / #[Secret] / interp ${} / 7.10 parse-env char concat.
# Does not rewrite Domain Expert product logic (c_emit_secret_guard.oo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

FIX="$TMPDIR/s71_guard_scan_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT

# --- fixtures (all under TMPDIR; never pollute repo root) ---

# FAIL-CLOSED: SECRET ident at println must refuse
cat >"$FIX/println_ident.oo" <<'EOF'
// 7.1 path-A: bare SECRET IDENT at println must refuse
// SECRET: tok
pub fn main() {
    let tok = "s3cret";
    println(tok);
}
EOF

# FAIL-CLOSED: SECRET ident as write_file payload must refuse
cat >"$FIX/write_file_payload.oo" <<'EOF'
// 7.1 path-A: SECRET payload IDENT at write_file must refuse
// SECRET: tok
pub fn main(fs: &FsCap) {
    let tok = "s3cret";
    let path = "/tmp/s71_guard_scan_should_not_write";
    let r = write_file(fs, path, tok);
    println(1);
}
EOF

# NO-FP: string literal spelling of SECRET name must not refuse
cat >"$FIX/println_string_lit.oo" <<'EOF'
// 7.1 path-A: println("tok") is a string literal, not IDENT — no false positive
// SECRET: tok
pub fn main() {
    println("tok");
}
EOF

# --- helpers ---

expect_refuse() {
  local src="$1"
  local label="$2"
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
  pass "fail-closed: $label"
  return 0
}

expect_ok() {
  local src="$1"
  local label="$2"
  local out="$FIX/${label}.out"
  set +e
  "$OODAC" emit-c "$src" >"$out" 2>&1
  local rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    bad "no-FP rail refused (want OK): $label rc=$rc"
    head -8 "$out" >&2 || true
    return 1
  fi
  if grep -qE $'ERR\tsecret' "$out"; then
    bad "no-FP rail has ERR secret: $label"
    return 1
  fi
  pass "no-FP: $label"
  return 0
}

# --- rails ---

expect_refuse "$FIX/println_ident.oo" "secret_tok_println_ident" || true
expect_refuse "$FIX/write_file_payload.oo" "secret_tok_write_file_payload" || true
expect_ok "$FIX/println_string_lit.oo" "secret_tok_println_string_lit" || true

if [[ $fail -ne 0 ]]; then
  echo "s71_guard_scan_smoke: FAILED" >&2
  exit 1
fi
echo "s71_guard_scan_smoke: ALL OK"
echo "RESIDUAL full IFC / #[Secret] attr / string-interp \${} deep taint — not claimed closed"
echo "RESIDUAL 7.10: c_parse_secret_env still char-by-char name concat (not this OWN)"
echo "RESIDUAL interproc SECRET beyond path-A __fr_secret__ (cross-file return taint)"
exit 0
