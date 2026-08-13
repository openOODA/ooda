#!/usr/bin/env bash
# Prove authorized residual packs exist and their listed smokes pass.
# Does not run unrelated *_residual_smoke.sh (e.g. F26 harness).
set -euo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"
[[ -f bootstrap/RESIDUAL_PACKS.oot ]]
n=$(grep -c '_RESIDUAL_ALPHA' bootstrap/RESIDUAL_PACKS.oot || true)
[[ "$n" -ge 20 ]]
fail=0
while read -r smoke; do
  [[ -z "$smoke" ]] && continue
  s="scripts/$smoke"
  if [[ ! -f "$s" ]]; then
    echo "FAIL missing $s" >&2
    fail=1
    continue
  fi
  if ! bash "$s" >/dev/null; then
    echo "FAIL $s" >&2
    fail=1
  fi
done < <(grep -oE '`[a-z0-9_]+_residual_smoke\.sh`' bootstrap/RESIDUAL_PACKS.oot | tr -d '`' | sort -u)
# also listed floor smoke
if [[ -f scripts/residual_path_a_floor_smoke.sh ]]; then
  bash scripts/residual_path_a_floor_smoke.sh >/dev/null || { echo FAIL residual_path_a_floor_smoke.sh >&2; fail=1; }
fi
[[ $fail -eq 0 ]]
echo "residual_packs_index_smoke: PASSED ($n packs indexed)"
