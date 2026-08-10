#!/usr/bin/env bash
# residual honesty smoke — generated pack
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
DOC="bootstrap/CONCURRENCY.md"
MARKER="CONCURRENCY_RESIDUAL_ALPHA"
FIX="fixtures/concurrency_marker.oo"
FIXLINE="CONCURRENCY: residual"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
[[ -f "$DOC" ]] || { echo "FAIL missing $DOC" >&2; exit 1; }
grep -q "$MARKER" "$DOC" && pass "doc marker $MARKER" || bad "doc marker $MARKER"
if grep -qiE 'shipped as enforced|fully enforced|product green|sandbox shipped' "$DOC" | grep -v residual >/dev/null 2>&1; then
  :
fi
# ban affirmative shipped claims without residual/not
if grep -nEi 'fully shipped|enforced in product|sandbox is live' "$DOC" | grep -viE 'not |residual|do not|never|no ' >/dev/null; then
  bad "doc may claim shipped without residual"
else
  pass "doc does not claim product-green enforce"
fi
grep -qiE 'fail-closed residual|Fail-closed residual' "$DOC" && pass "fail-closed residual wording" || bad "fail-closed residual wording"
[[ -f "$FIX" ]] || bad "missing fixture $FIX"
grep -q "$FIXLINE" "$FIX" && pass "fixture marker" || bad "fixture marker $FIXLINE"
# dual claim scan residual surface
hits=$(grep -rn --include='*.md' -E 'shipped as enforced|fully sealed shipped' bootstrap 2>/dev/null | grep -v "$(basename "$DOC")" | grep -vi residual | head -5 || true)
if [[ -n "$hits" ]]; then
  echo "$hits" | head -3
  bad "bootstrap residual surface dual-claim"
else
  pass "no dual-claim on residual surface"
fi
if grep -q "$(basename "$0")" scripts/ci_product.sh; then
  pass "ci_product wire"
else
  bad "ci_product missing $(basename "$0")"
fi
if [[ $fail -ne 0 ]]; then echo "$(basename "$0" .sh): FAILED" >&2; exit 1; fi
echo "$(basename "$0" .sh): PASSED"
exit 0
