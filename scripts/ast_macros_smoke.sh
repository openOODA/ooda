#!/usr/bin/env bash
# job: AST macros and metaprogramming smoke test
# in:  oodac, std/macro.oo, oodac/ast_macros.oo, oodac/macro_expand.oo, fixtures/ast_macros_fixture.oo
# out: exit 0 if AST macros pass checks
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo ERR_NO_OODAC >&2; exit 1; }

for m in "$ROOT/oodac/ast_macros.oo" "$ROOT/oodac/macro_expand.oo" "$ROOT/std/macro.oo" "$ROOT/fixtures/ast_macros_fixture.oo"; do
  set +e
  "$OODAC" check "$m" >"$TMPDIR/macro_ck.out" 2>"$TMPDIR/macro_ck.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    bad "check $m"
    head -12 "$TMPDIR/macro_ck.err" 2>/dev/null || true
  else
    pass "check $m"
  fi
done

if [[ $fail -eq 0 ]]; then
  echo "ast_macros_smoke.sh: PASSED"
  exit 0
else
  echo "ast_macros_smoke.sh: FAILED"
  exit 1
fi
