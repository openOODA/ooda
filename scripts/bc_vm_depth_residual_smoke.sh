#!/usr/bin/env bash
set -euo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
fail=0
grep -q "BC_VM_DEPTH_RESIDUAL_ALPHA" bootstrap/BC_VM_DEPTH.oot || fail=1
grep -qiE 'fail-closed residual|Fail-closed residual|not claim|not product' bootstrap/BC_VM_DEPTH.oot || fail=1
grep -q "$(basename "$0")" scripts/ci_product.sh || fail=1
if [[ $fail -ne 0 ]]; then echo FAILED; exit 1; fi
echo "$(basename $0 .sh): PASSED"
