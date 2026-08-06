#!/usr/bin/env bash
# job: product ooda patch — structured replace_fn only (fail-closed)
# in:  args forwarded to ooda_patch.py; optional OODAC_BIN for --check
# out: exit 0 on success; 2 usage/safety; 1 other
# SECURITY: never shell-evals patch body; path checks in python
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

PY="$ROOT/scripts/ooda_patch.py"
if [[ ! -f "$PY" ]]; then
  echo "ERR	patch	missing $PY" >&2
  exit 1
fi

if [[ -z "${OODAC_BIN:-}" || ! -x "${OODAC_BIN:-}" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then
    export OODAC_BIN="$ROOT/oodac/oodac"
  elif [[ -x ./oodac/oodac ]]; then
    export OODAC_BIN=./oodac/oodac
  fi
fi

exec python3 "$PY" "$@"
