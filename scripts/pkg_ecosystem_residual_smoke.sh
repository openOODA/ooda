#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; cd "$ROOT"
fail=0
pass(){ echo "OK $*"; }; bad(){ echo "FAIL $*" >&2; fail=1; }
DOC="bootstrap/PKG_ECOSYSTEM.oot"; MARKER="PKG_ECOSYSTEM_RESIDUAL_ALPHA"; FIX="fixtures/pkg_ecosystem_marker.oo"; FIXLINE="PKG_ECOSYSTEM: residual"
[[ -f "$DOC" ]] || { echo missing $DOC; exit 1; }
grep -q "$MARKER" "$DOC" && pass "marker" || bad "marker"
grep -qiE 'fail-closed residual|Fail-closed residual' "$DOC" && pass "fail-closed" || bad "fail-closed"
grep -qiE 'do \*\*not\*\* claim|What we do \*\*not\*\* claim|not claim' "$DOC" && pass "non-claims" || bad "non-claims"
[[ -f "$FIX" ]] && grep -q "$FIXLINE" "$FIX" && pass "fixture" || bad "fixture"
grep -q "$(basename "$0")" scripts/ci_product.sh && pass "ci wire" || bad "ci wire"
if [[ $fail -ne 0 ]]; then echo "$(basename "$0" .sh): FAILED" >&2; exit 1; fi
echo "$(basename "$0" .sh): PASSED"
