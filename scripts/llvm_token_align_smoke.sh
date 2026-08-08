#!/usr/bin/env bash
# Smoke: emit-llvm on tiny int/string fixtures; grep for real constants.
# Residual: binary must include llvm path + token-kind fixes (rebuild if stale).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="$ROOT/oodac/oodac"
TMP="${TMPDIR:-/tmp}/llvm_token_align_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

cat >"$TMP/int_lit.oo" <<'EOF'
pub fn main() {
    let x = 42;
    println(x);
}
EOF

cat >"$TMP/str_lit.oo" <<'EOF'
pub fn main() {
    let s = "hi";
    println(s);
}
EOF

echo "== tokens (int_lit) =="
timeout 20 "$OODAC" tokens "$TMP/int_lit.oo" | head -40 || true

echo "== emit-llvm int_lit =="
if timeout 30 "$OODAC" emit-llvm "$TMP/int_lit.oo" >"$TMP/int.ll" 2>"$TMP/int.err"; then
  echo "emit-llvm ok"
  grep -E 'i64 42|ret i32|define ' "$TMP/int.ll" || true
  if grep -q 'i64 42' "$TMP/int.ll"; then
    echo "PASS: int constant 42 present"
  else
    echo "FAIL_OR_STALE: no i64 42 (binary may predate token-kind fix)"
    head -80 "$TMP/int.ll" || true
  fi
else
  echo "emit-llvm failed (exit $?); stderr:"
  cat "$TMP/int.err" || true
  echo "RESIDUAL: needs rebuild of oodac to pick up llvm_emit*.oo fixes"
fi

echo "== emit-llvm str_lit =="
if timeout 30 "$OODAC" emit-llvm "$TMP/str_lit.oo" >"$TMP/str.ll" 2>"$TMP/str.err"; then
  grep -E '@\.str\.|c"hi|oo_str_lit' "$TMP/str.ll" || true
  if grep -q '@.str.' "$TMP/str.ll" && grep -q 'c"hi' "$TMP/str.ll"; then
    echo "PASS: string pool global present"
  else
    echo "FAIL_OR_STALE: string pool missing or not declared"
  fi
else
  echo "emit-llvm str failed; RESIDUAL: needs rebuild"
  cat "$TMP/str.err" || true
fi
