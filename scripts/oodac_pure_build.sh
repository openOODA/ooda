#!/usr/bin/env bash
# job: pure multi-module oodac build (emit each .oo, link once) — no stage-0 host
# in:  <main.oo> <out_bin>
# out: native binary via emit-c + gcc + chs_rt only
# link recipe: Backend-C (see bootstrap/FLOOR.md) — swap here for other floors later
# Notes:
#  - Forward prototypes for all fns so use-before-def across modules is OK
#  - Nested imports + cycle/missing fail-closed (parity with load_import.oo)
#  - Never uses $OODA host soft-pass
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

if [[ ! -f "$MAIN" ]]; then
  if [[ -f "$ROOT/$MAIN" ]]; then MAIN="$ROOT/$MAIN"; fi
fi
if [[ ! -f "$MAIN" ]]; then
  echo "ERR_MISSING $MAIN" >&2
  exit 1
fi
DIR="$(cd "$(dirname "$MAIN")" && pwd)"
BASE="$(basename "$MAIN")"
MAIN_ABS="$DIR/$BASE"

# Collect modules (DFS, cycle/missing fail-closed). Order: deps first, main last.
MODS=()
declare -A SEEN=()
declare -A STACK=()

collect() {
  local path="$1"
  local abs
  if [[ "$path" = /* ]]; then abs="$path"
  else abs="$(cd "$(dirname "$path")" && pwd)/$(basename "$path")"
  fi
  if [[ -n "${STACK[$abs]:-}" ]]; then
    echo "ERR_IMPORT_CYCLE $abs" >&2
    exit 1
  fi
  if [[ -n "${SEEN[$abs]:-}" ]]; then
    return 0
  fi
  if [[ ! -f "$abs" ]]; then
    echo "ERR_MISSING $abs" >&2
    exit 1
  fi
  STACK[$abs]=1
  local dir
  dir="$(cd "$(dirname "$abs")" && pwd)"
  local imp
  while IFS= read -r imp; do
    [[ -z "$imp" ]] && continue
    if [[ "$imp" = /* ]]; then
      collect "$imp"
    else
      collect "$dir/$imp"
    fi
  done < <(grep -E '^import "' "$abs" 2>/dev/null | sed -n 's/^import "\(.*\)";.*/\1/p' || true)
  unset 'STACK[$abs]'
  SEEN[$abs]=1
  MODS+=("$abs")
}

collect "$MAIN_ABS"

FN_DEF='^(void|int|long long|OoStr|OoSList|OoIList|OoResS|OoResV) [A-Za-z_].*\) \{'
MCS=()
for src in "${MODS[@]}"; do
  mc="$TMP/$(echo "$src" | tr '/.' '__').c"
  EMIT_NO_CONCAT=1 timeout 60 "$OODAC_BIN" emit-c "$src" >"$mc" 2>/dev/null || true
  if [[ ! -s "$mc" ]] || ! grep -qE "$FN_DEF" "$mc"; then
    echo "ERR_EMIT $src" >&2
    exit 1
  fi
  if grep -qE $'^ERR\tc_emit' "$mc"; then
    echo "ERR_EMIT_LINE $src" >&2
    grep -E $'^ERR\tc_emit' "$mc" >&2 || true
    exit 1
  fi
  MCS+=("$mc")
done

# Preamble = lines before first function def in first module
awk "/$FN_DEF/{exit} {print}" "${MCS[0]}" >"$TMP/preamble.c"
# Forward prototypes from ALL modules (use-before-def across modules)
: >"$TMP/protos.c"
for mc in "${MCS[@]}"; do
  grep -E "$FN_DEF" "$mc" | sed 's/ {$/;/' >>"$TMP/protos.c" || true
done
# Function bodies from ALL modules
: >"$TMP/bodies.c"
for mc in "${MCS[@]}"; do
  awk "/$FN_DEF/{p=1} p" "$mc" >>"$TMP/bodies.c"
done

cat "$TMP/preamble.c" "$TMP/protos.c" "$TMP/bodies.c" >"$TMP/all.c"

if ! grep -q 'int main' "$TMP/all.c"; then
  echo "ERR_NO_MAIN" >&2
  exit 1
fi

gcc -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/all.c" -lm -o "$OUT"
test -x "$OUT"
echo OK_PURE_MULTI
rm -rf "$TMP"
