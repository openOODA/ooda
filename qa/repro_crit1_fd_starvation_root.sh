#!/usr/bin/env bash
# Repro 1 for CRIT Finding 1: FD starvation during product script ROOT resolution
# Verifies that ulimit -n starvation causes explicit fail-closed exit 1 with ERR_ROOT_INVALID,
# rather than silently continuing with empty ROOT and producing false OK output.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd)" || true
[[ -n "$ROOT" && -d "$ROOT" ]] || { echo "ERR_ROOT_INVALID" >&2; exit 1; }

OODA="${OODA:-$ROOT/bin/ooda}"
[[ -x "$OODA" ]] || { echo "ERR_MISSING_OODA: $OODA" >&2; exit 1; }

set +e
out=$(ulimit -n 4; "$OODA" check "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc=$?
set -e

if [[ $rc -eq 0 ]] && echo "$out" | grep -qE '^OK$'; then
  echo "REPRO CRIT1 FAIL: false OK exit status 0 under ulimit -n 4 (out=$out)" >&2
  exit 1
fi

if [[ $rc -ne 0 ]] && echo "$out" | grep -qE 'ERR_ROOT_INVALID|Too many open files'; then
  echo "REPRO CRIT1 CONFIRMED: fail-closed with exit $rc and explicit error message"
  exit 0
else
  echo "REPRO CRIT1 UNEXPECTED: exit=$rc out=$out" >&2
  exit 1
fi
