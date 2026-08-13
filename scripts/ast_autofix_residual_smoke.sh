#!/usr/bin/env bash
set -euo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
fail=0
grep -q "AST_AUTOFIX_RESIDUAL_ALPHA" bootstrap/AST_AUTOFIX.oot || fail=1
grep -qiE 'fail-closed residual|Fail-closed residual|not claim|not product' bootstrap/AST_AUTOFIX.oot || fail=1
grep -q "$(basename "$0")" scripts/ci_product.sh || fail=1
if [[ $fail -ne 0 ]]; then echo FAILED; exit 1; fi
echo "$(basename $0 .sh): PASSED"
