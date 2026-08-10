#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/bin" "$ROOT/oodac"

SEED="$ROOT/bootstrap/seed/oodac"
STAGE1="$ROOT/oodac/stage1_noarc"
STAGE2="$ROOT/oodac/oodac"

echo "=== STAGE 1: seed builds stage1_noarc (PURE_NO_ARC=1) ==="
(cd "$ROOT" && env -u OODA PURE_NO_ARC=1 OODAC_BIN="$SEED" "$SEED" build "$ROOT/oodac/main.oo" "$STAGE1")
if [[ ! -x "$STAGE1" ]]; then
  echo "FAIL: STAGE1" >&2
  exit 1
fi

echo "=== STAGE 2: stage1_noarc builds oodac (PURE_NO_ARC=0) ==="
(cd "$ROOT" && env -u OODA PURE_NO_ARC=0 OODAC_BIN="$STAGE1" "$STAGE1" build "$ROOT/oodac/main.oo" "$STAGE2")
if [[ ! -x "$STAGE2" ]]; then
  echo "FAIL: STAGE2" >&2
  exit 1
fi

echo "=== Build pure CLI with STAGE2 (PURE_NO_ARC=0) ==="
CLI_OUT="$ROOT/bin/ooda"
(cd "$ROOT" && env -u OODA PURE_NO_ARC=0 OODAC_BIN="$STAGE2" "$STAGE2" build "$ROOT/cli/main.oo" "$CLI_OUT")

echo "=== arc_smoke.sh ==="
export OODAC_BIN="$STAGE2"
bash "$ROOT/scripts/arc_smoke.sh"
echo "DONE!"
