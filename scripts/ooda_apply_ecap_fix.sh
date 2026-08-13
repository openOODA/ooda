#!/usr/bin/env bash
# Product entry: bounded E_CAP structural auto-fix (no shell-eval of diagnostics)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
if [[ -z "${OODAC_BIN:-}" || ! -x "${OODAC_BIN:-}" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then export OODAC_BIN="$ROOT/oodac/oodac"
  elif [[ -x ./oodac/oodac ]]; then export OODAC_BIN=./oodac/oodac
  fi
fi
# Dispatcher: E_CAP, E_TC undefined-var (M158), E_HITL pause (M165)
exec python3 "$ROOT/scripts/ooda_apply_fix.py" "$@"
