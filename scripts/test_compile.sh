#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "=== STAGE 1: seed builds stage1_noarc (PURE_NO_ARC=1) ==="
export PURE_NO_ARC=1
export PURE_SKIP_CHECK=1
OODAC_BIN="$ROOT/bootstrap/seed/oodac" bash "$ROOT/scripts/my_pure_build.sh" oodac/main.oo "$ROOT/oodac/stage1_noarc"

echo "=== STAGE 2: stage1_noarc builds oodac (PURE_NO_ARC=0) ==="
export PURE_NO_ARC=0
export PURE_SKIP_CHECK=1
OODAC_BIN="$ROOT/oodac/stage1_noarc" bash "$ROOT/scripts/my_pure_build.sh" oodac/main.oo "$ROOT/oodac/oodac"

echo "=== arc_smoke.sh ==="
export OODAC_BIN="$ROOT/oodac/oodac"
bash "$ROOT/scripts/arc_smoke.sh"
echo "DONE!"
