#!/usr/bin/env bash
# job: build product ooda + oodac without rustc/cargo (P3 / B1 path)
# in:  SEED_OODAC (or existing oodac/oodac|oodac2) + gcc + sources
# out: oodac/oodac, bin/ooda (pure .oo CLI)
# Anti: never invokes cargo/rustc; never OK_HOST host soft-pass
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/bin" "$ROOT/oodac"

# Never invoke cargo/rustc on this path (machine may still have them installed).

STAGE1="$ROOT/oodac/oodac"
SEED_SRC="${SEED_OODAC:-}"
if [[ -z "$SEED_SRC" || ! -x "$SEED_SRC" ]]; then
  if [[ -x "$ROOT/oodac/oodac2" ]]; then
    SEED_SRC="$ROOT/oodac/oodac2"
  elif [[ -x "$ROOT/oodac/oodac" ]]; then
    SEED_SRC="$ROOT/oodac/oodac"
  else
    echo "ERR_NO_SEED: set SEED_OODAC to a pure oodac binary" >&2
    echo "  (first seed: obtain prebuilt oodac; host Rust seed retired)" >&2
    exit 1
  fi
fi
# Always copy seed aside so rm STAGE1 cannot unlink the live seed.
SEED="$TMPDIR/bootstrap_seed_oodac"
cp -a "$SEED_SRC" "$SEED"
chmod +x "$SEED"
echo "bootstrap_no_cargo: seed=$SEED (from $SEED_SRC)"

# 1) Rebuild oodac from sources (pure multi)
rm -f "$STAGE1"
echo "=== seed builds oodac ==="
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "$SEED" build "$ROOT/oodac/main.oo" "$STAGE1")
if [[ ! -x "$STAGE1" ]]; then
  echo "FAIL: seed did not produce $STAGE1" >&2
  exit 1
fi

# 2) Build product CLI from cli/main.oo
CLI_OUT="$ROOT/bin/ooda"
rm -f "$CLI_OUT"
echo "=== stage-1 builds pure .oo product CLI ==="
(cd "$ROOT" && env -u OODA OODAC_BIN="$STAGE1" "$STAGE1" build "$ROOT/cli/main.oo" "$CLI_OUT")
if [[ ! -x "$CLI_OUT" ]]; then
  echo "FAIL: pure CLI missing at $CLI_OUT" >&2
  exit 1
fi

# 3) Smoke product CLI without cargo
echo "=== smoke product bin/ooda ==="
"$CLI_OUT" version | tee "$TMPDIR/bootstrap_ver.txt"
grep -q '0.182.0-alpha' "$TMPDIR/bootstrap_ver.txt"
"$CLI_OUT" check "$ROOT/fixtures/chs_list_string.oo" | tee "$TMPDIR/bootstrap_chk.txt"
grep -qE '^OK' "$TMPDIR/bootstrap_chk.txt"
"$CLI_OUT" run "$ROOT/fixtures/chs_list_string.oo" | tee "$TMPDIR/bootstrap_run.txt"
grep -q '2' "$TMPDIR/bootstrap_run.txt"

# Anti: no .rs required on this path (report RS for honesty)
RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' 2>/dev/null | wc -l)
echo "RS_COUNT=$RS (bootstrap does not need rustc; B0 wants 0)"
echo "bootstrap_no_cargo: PASSED"
echo "  oodac: $STAGE1"
echo "  ooda:  $CLI_OUT"
exit 0
