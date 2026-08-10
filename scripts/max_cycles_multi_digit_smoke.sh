#!/usr/bin/env bash
# M138: multi-digit // MAX_CYCLES: N lands in emit (N=50 fixture)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
FIX="$ROOT/fixtures/max_cycles_recursion_pass.oo"
grep -qE 'MAX_CYCLES:[[:space:]]*[0-9]{2,}' "$FIX" || { echo "FAIL fixture not multi-digit" >&2; exit 1; }
out="$("$OODAC" emit-c "$FIX" 2>&1)" || { echo "FAIL emit" >&2; exit 1; }
echo "$out" | grep -q '#define OO_MC_LIMIT 50' || { echo "FAIL missing OO_MC_LIMIT 50" >&2; exit 1; }
echo "OK max_cycles multi-digit N=50 in emit"
