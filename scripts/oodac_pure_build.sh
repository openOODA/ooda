#!/usr/bin/env bash
# job: pure multi-module oodac build (emit each .oo, link once) — no stage-0 host
# in:  <main.oo> <out_bin>
# out: native binary via emit-c + gcc + chs_rt only
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAIN="${1:?main.oo}"
OUT="${2:?out_bin}"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/oodac_pure_$$"
mkdir -p "$TMP"

if [[ ! -x "$OODAC_BIN" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then OODAC_BIN="$ROOT/oodac/oodac"
  elif [[ -x "$ROOT/oodac/main" ]]; then OODAC_BIN="$ROOT/oodac/main"
  else echo "ERR_NO_OODAC" >&2; exit 1
  fi
fi

# Resolve main path relative to ROOT when possible
if [[ ! -f "$MAIN" ]]; then
  if [[ -f "$ROOT/$MAIN" ]]; then MAIN="$ROOT/$MAIN"; fi
fi
DIR="$(cd "$(dirname "$MAIN")" && pwd)"
BASE="$(basename "$MAIN")"

# Collect modules: imports then main (order: deps first)
mapfile -t MODS < <(grep -E '^import "' "$MAIN" | sed -n 's/^import "\(.*\)";/\1/p')
MODS+=("$BASE")

# Emit each module alone (no full-tree concat — that stalls token emit)
: >"$TMP/all.c"
first=1
for m in "${MODS[@]}"; do
  src="$DIR/$m"
  [[ -f "$src" ]] || { echo "ERR_MISSING $src" >&2; exit 1; }
  mc="$TMP/$(echo "$m" | tr '/.' '__').c"
  # EMIT_NO_CONCAT: single-file emit only
  EMIT_NO_CONCAT=1 timeout 60 "$OODAC_BIN" emit-c "$src" >"$mc" 2>/dev/null || true
  if [[ ! -s "$mc" ]] || ! grep -qE '^void |^int main' "$mc"; then
    echo "ERR_EMIT $src" >&2
    exit 1
  fi
  if grep -qE $'^ERR\tc_emit' "$mc"; then
    echo "ERR_EMIT_LINE $src" >&2
    grep -E $'^ERR\tc_emit' "$mc" >&2 || true
    exit 1
  fi
  if [[ $first -eq 1 ]]; then
    cat "$mc" >>"$TMP/all.c"
    first=0
  else
    # drop preamble; keep function defs only
    awk '/^void |^int main/{p=1} p' "$mc" >>"$TMP/all.c"
  fi
done

if ! grep -q 'int main' "$TMP/all.c"; then
  echo "ERR_NO_MAIN" >&2
  exit 1
fi

gcc -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/all.c" -lm -o "$OUT"
test -x "$OUT"
echo OK_PURE_MULTI
rm -rf "$TMP"
