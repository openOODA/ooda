#!/usr/bin/env bash
set -euo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
grep -q "LLVM_SMOKE_RESIDUAL_ALPHA" bootstrap/LLVM_SMOKE.oot
grep -qiE 'not.*production floor|smoke depth|Fail-closed residual|fail-closed residual' bootstrap/LLVM_SMOKE.oot
grep -q "$(basename "$0")" scripts/ci_product.sh
echo "$(basename $0 .sh): PASSED"
