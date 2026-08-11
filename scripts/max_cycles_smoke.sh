#!/usr/bin/env bash
# job: M48 MaxCycles path A product smoke (delegates to enforce + residual rails)
# in:  max_cycles_enforce_smoke + max_cycles_residual_smoke
# out: exit 0 if both green
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fail=0

run() {
  local s="$1"
  if bash "$ROOT/scripts/$s"; then
    echo "OK $s"
  else
    echo "FAIL $s" >&2
    fail=1
  fi
}

run max_cycles_enforce_smoke.sh
run max_cycles_residual_smoke.sh

if [[ $fail -ne 0 ]]; then
  echo "max_cycles_smoke: FAILED" >&2
  exit 1
fi
echo "max_cycles_smoke: PASSED"
exit 0
