#!/usr/bin/env bash
# job: residual honesty for multi-arg pure fuzz (arity≥4 residual; arity-2/3 In)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
fail=0
pass(){ echo "OK $*"; }; bad(){ echo "FAIL $*" >&2; fail=1; }
DOC="bootstrap/MULTI_ARG_FUZZ.md"
MARKER="MULTI_ARG_FUZZ_RESIDUAL_ALPHA"
FIX="fixtures/multi_arg_fuzz_marker.oo"
FIXLINE="MULTI_ARG_FUZZ: residual"
[[ -f "$DOC" ]] || { echo missing $DOC; exit 1; }
grep -q "$MARKER" "$DOC" && pass "marker" || bad "marker"
grep -qiE 'fail-closed residual|Fail-closed residual' "$DOC" && pass "fail-closed" || bad "fail-closed"
grep -qiE 'do \*\*not\*\* claim|What we do \*\*not\*\* claim|not claim' "$DOC" && pass "non-claims" || bad "non-claims"
# In surface must be named (no residual doc lie that arity-3 is still Out)
grep -qiE 'arity-3 multi-arg In|arity-2/3 In' "$DOC" && pass "arity-3 In named" || bad "arity-3 In named"
grep -qiE 'Bool arity-2|bool arity-2' "$DOC" && pass "Bool arity-2 In named" || bad "Bool arity-2 In named"
grep -qiE 'String arity-2|string arity-2' "$DOC" && pass "String arity-2 In named" || bad "String arity-2 In named"
# Residual boundary is arity≥4 (not stale arity≥3 residual claim alone)
grep -qiE 'Arity ≥4|arity≥4|arity>=4' "$DOC" && pass "arity≥4 residual named" || bad "arity≥4 residual named"
# Stale lie: claiming arity≥3 residual as the only residual while also In is ok if In named;
# reject docs that only say arity≥3 residual without In.
if grep -qiE 'arity≥3.*residual|arity>=3.*residual' "$DOC" \
  && ! grep -qiE 'arity-3 multi-arg In|arity-2/3 In' "$DOC"; then
  bad "stale arity≥3 residual without In"
else
  pass "no stale arity≥3 residual lie"
fi
[[ -f "$FIX" ]] && grep -q "$FIXLINE" "$FIX" && pass "fixture" || bad "fixture"
grep -q "$(basename "$0")" scripts/ci_product.sh && pass "ci wire" || bad "ci wire"
if [[ $fail -ne 0 ]]; then echo "$(basename "$0" .sh): FAILED" >&2; exit 1; fi
echo "$(basename "$0" .sh): PASSED"
