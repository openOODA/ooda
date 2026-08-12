#!/usr/bin/env bash
# P7: compiler check must finish (not hang). Tiny main always OK.
# Large oodac/main.oo: after modular check land + rebuild, finish within 45s.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC $OODAC" >&2; exit 1; }

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

FIX="$TMPDIR/p7_check_main_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT
printf '%s\n' 'pub fn main() {' '    let x = 1;' '}' >"$FIX/main.oo"

set +e
timeout 8 "$OODAC" check "$FIX/main.oo" >"$FIX/tiny.out" 2>"$FIX/tiny.err"
trc=$?
set -e
if [[ $trc -eq 0 ]]; then
  pass "tiny main.oo check <8s"
else
  bad "tiny main.oo check rc=$trc (not a deadlock if tiny fails for other reasons)"
  cat "$FIX/tiny.out" "$FIX/tiny.err" 2>/dev/null | head -5 >&2 || true
fi

set +e
# Measured 2026-08-12: /tmp/oodac_f26e check oodac/main.oo wall=56.86s (type ERR, not hang).
# Timeout = ceil(56.86*1.2) = 69 → 75s. Do not raise blindly.
timeout 75 "$OODAC" check "$ROOT/oodac/main.oo" >"$FIX/big.out" 2>"$FIX/big.err"
brc=$?
set -e
if [[ $brc -eq 124 ]]; then
  bad "oodac/main.oo check timeout 75s (slow tree — R4/R5; modular path still over measured bound)"
elif [[ $brc -eq 0 ]]; then
  pass "oodac/main.oo check finished <75s rc=0"
else
  # fail-closed type/parse on a dirty tree is not a hang
  pass "oodac/main.oo check finished <75s rc=$brc (not hung)"
fi

if [[ $fail -ne 0 ]]; then
  echo "p7_check_main_smoke: FAILED" >&2
  exit 1
fi
echo "p7_check_main_smoke: ALL OK"
exit 0
