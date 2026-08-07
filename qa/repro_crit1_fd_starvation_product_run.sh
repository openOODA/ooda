#!/usr/bin/env bash
# Repro 2 for CRIT Finding 1: FD starvation during product CLI run dispatch
# Verifies that ooda run fails-closed cleanly with non-zero exit code when subshell/pipe creation fails.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd)" || true
[[ -n "$ROOT" && -d "$ROOT" ]] || { echo "ERR_ROOT_INVALID" >&2; exit 1; }

OODA="${OODA:-$ROOT/bin/ooda}"
[[ -x "$OODA" ]] || { echo "ERR_MISSING_OODA: $OODA" >&2; exit 1; }

set +e
out=$(ulimit -n 4; "$OODA" run "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc=$?
set -e

if [[ $rc -eq 0 ]]; then
  echo "REPRO CRIT1 RUN FAIL: ooda run returned exit status 0 under ulimit -n 4 (out=$out)" >&2
  exit 1
fi

if echo "$out" | grep -qE 'ERR_ROOT_INVALID|Too many open files|cannot make pipe'; then
  echo "REPRO CRIT1 RUN CONFIRMED: ooda run failed closed with exit code $rc"
  exit 0
else
  echo "REPRO CRIT1 RUN UNEXPECTED: exit=$rc out=$out" >&2
  exit 1
fi
