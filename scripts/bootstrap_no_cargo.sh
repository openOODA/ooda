#!/usr/bin/env bash
# job: build product oodac + bin/ooda from seed
# in:  SEED_OODAC (or existing oodac/oodac|oodac2) + gcc + sources
# out: oodac/oodac, bin/ooda (pure .oo CLI)
set -euo pipefail
# Default PURE_NO_ARC=0: keep retain/release in pure-built C (no strip required).
# Runtime release is leak-safe (does not free) until emit ARC is reclaim-correct.
# Optional PURE_NO_ARC=1 still strips if debugging seed-era heap issues.
export PURE_NO_ARC="${PURE_NO_ARC:-0}"
export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-1}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/bin" "$ROOT/oodac"

STAGE1="$ROOT/oodac/oodac"
# Prefer pinned cold seed over tree stage-1: a half-built/corrupt oodac/oodac
# must not become the emit host (ci deletes STAGE1 mid-bootstrap; SEGV host fails closed).
SEED_SRC="${SEED_OODAC:-}"
if [[ -z "$SEED_SRC" || ! -x "$SEED_SRC" ]]; then
  if [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then
    SEED_SRC="$ROOT/bootstrap/seed/oodac"
  elif [[ -x "$ROOT/oodac/oodac2" ]]; then
    SEED_SRC="$ROOT/oodac/oodac2"
  elif [[ -x "$ROOT/dist/ooda-v0.183.0-alpha-linux-x86_64/oodac/oodac" ]]; then
    SEED_SRC="$ROOT/dist/ooda-v0.183.0-alpha-linux-x86_64/oodac/oodac"
  elif [[ -x "$ROOT/oodac/oodac" ]]; then
    SEED_SRC="$ROOT/oodac/oodac"
  else
    echo "ERR_NO_SEED: set SEED_OODAC to a pure oodac binary" >&2
    echo "  (expected $ROOT/bootstrap/seed/oodac)" >&2
    exit 1
  fi
fi
# Always copy seed aside so rm STAGE1 cannot unlink the live seed.
SEED="$TMPDIR/bootstrap_seed_oodac"
rm -f "$SEED"
cp -a "$SEED_SRC" "$SEED"
chmod +x "$SEED"
echo "bootstrap: seed=$SEED (from $SEED_SRC)"

# Residual honesty: stage-1 pure oodac can SEGV as *emit host* on some CLI
# modules (e.g. cli/product_sh.oo → c_emit_let/oo_str_concat). Cold seed is the
# trusted emit host under PURE_NO_ARC until M2 stage2 is green. Stage-1 is still
# the product oodac binary (tokens/ast/check/emit for fixtures that work).
# stage1 `run` is char_at/ARC-hostile under PURE_NO_ARC — native prove = build+exec.
#
# 1) Rebuild oodac from sources (pure multi) — seed is emit host
rm -f "$STAGE1"
echo "=== seed builds oodac (emit host=seed) ==="
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "$SEED" build "$ROOT/oodac/main.oo" "$STAGE1")
if [[ ! -x "$STAGE1" ]]; then
  echo "FAIL: seed did not produce $STAGE1" >&2
  exit 1
fi

# 2) Build product CLI from cli/main.oo — seed is emit host (not stage-1)
CLI_OUT="$ROOT/bin/ooda"
rm -f "$CLI_OUT"
echo "=== seed builds pure .oo product CLI (emit host=seed) ==="
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "$SEED" build "$ROOT/cli/main.oo" "$CLI_OUT")
if [[ ! -x "$CLI_OUT" ]]; then
  echo "FAIL: pure CLI missing at $CLI_OUT" >&2
  exit 1
fi

# 3) Smoke product CLI
echo "=== smoke product bin/ooda ==="
"$CLI_OUT" version | tee "$TMPDIR/bootstrap_ver.txt"
grep -q '0.183.0-alpha' "$TMPDIR/bootstrap_ver.txt"
"$CLI_OUT" check "$ROOT/fixtures/chs_list_string.oo" | tee "$TMPDIR/bootstrap_chk.txt"
grep -qE '^OK' "$TMPDIR/bootstrap_chk.txt"
SMOKE_BIN="$TMPDIR/bootstrap_chs_native"
rm -f "$SMOKE_BIN"
# Emit host = seed (same residual policy as pure builds above).
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "$CLI_OUT" build \
  "$ROOT/fixtures/chs_list_string.oo" -o "$SMOKE_BIN")
if [[ ! -x "$SMOKE_BIN" ]]; then
  echo "FAIL: product build did not produce $SMOKE_BIN" >&2
  exit 1
fi
"$SMOKE_BIN" | tee "$TMPDIR/bootstrap_run.txt"
grep -q '2' "$TMPDIR/bootstrap_run.txt"

# Product purity: report residual .rs count (B0 wants 0)
RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' 2>/dev/null | wc -l)
echo "RS_COUNT=$RS"
echo "bootstrap: PASSED"
echo "  oodac: $STAGE1"
echo "  ooda:  $CLI_OUT"
exit 0
