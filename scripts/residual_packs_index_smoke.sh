#!/usr/bin/env bash
set -euo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
[[ -f bootstrap/RESIDUAL_PACKS.md ]]
n=$(grep -c '_RESIDUAL_ALPHA' bootstrap/RESIDUAL_PACKS.md || true)
[[ "$n" -ge 20 ]]
# every residual smoke script must pass
fail=0
for s in scripts/*_residual_smoke.sh; do
  bash "$s" >/dev/null || { echo FAIL $s; fail=1; }
done
[[ $fail -eq 0 ]]
echo "residual_packs_index_smoke: PASSED ($n packs indexed)"
