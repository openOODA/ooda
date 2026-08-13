#!/usr/bin/env bash
# CAP-G5: product emit must not claim live seccomp-bpf / must not emit broken oo_seccomp_init
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-.ooda-cache/ooda-tmp}/cap_g5_smoke_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT
pass() { echo "OK $*"; }
fail() { echo "FAIL $*" >&2; exit 1; }
[[ -x "$OODAC" ]] || fail "oodac missing"

# Source honesty: no product oo_seccomp_init body
if grep -q 'static inline void oo_seccomp_init' oodac/c_emit_preamble.oo; then
  fail "c_emit_preamble still emits oo_seccomp_init body"
fi
grep -q 'CAP-G5 residual' oodac/c_emit_preamble.oo || fail "missing CAP-G5 residual comment in preamble"
pass "preamble residual honesty markers"

# Emit-c must not introduce sock_filter / BPF_STMT
printf '%s\n' 'fn main() {}' >"$TMP/m.oo"
"$OODAC" emit-c "$TMP/m.oo" >"$TMP/m.c" 2>/dev/null || fail "emit-c failed"
if grep -qE 'sock_filter|BPF_STMT|oo_seccomp_init|SECCOMP_MODE_FILTER' "$TMP/m.c"; then
  fail "emit-c still contains seccomp filter debris"
fi
grep -q 'CAP-G5 residual' "$TMP/m.c" || fail "emit-c missing CAP-G5 residual comment"
pass "emit-c free of seccomp-bpf filter (process-local caps only)"

if grep -q 'cap_g5_seccomp' scripts/ci_product.sh 2>/dev/null; then
  pass "ci_product soft-wire present"
else
  pass "ci_product optional: wire cap_g5_seccomp_honesty_smoke"
fi
echo "cap_g5_seccomp_honesty_smoke: PASSED"
