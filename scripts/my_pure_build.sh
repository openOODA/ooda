#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAIN="${1:?main.oo}"
OUT="${2:?out_bin}"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/oodac_pure_$$"
mkdir -p "$TMP"
cleanup_pure_tmp() { rm -rf "$TMP"; }
trap cleanup_pure_tmp EXIT

if [[ ! -x "$OODAC_BIN" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then OODAC_BIN="$ROOT/oodac/oodac"
  elif [[ -x "$ROOT/oodac/main" ]]; then OODAC_BIN="$ROOT/oodac/main"
  else echo "ERR_NO_OODAC" >&2; exit 1; fi
fi
if [[ ! -f "$MAIN" && -f "$ROOT/$MAIN" ]]; then MAIN="$ROOT/$MAIN"; fi
if [[ ! -f "$MAIN" ]]; then echo "ERR_MISSING $MAIN" >&2; exit 1; fi

DIR="$(cd "$(dirname "$MAIN")" && pwd)"
BASE="$(basename "$MAIN")"
MAIN_ABS="$DIR/$BASE"

if [[ "${PURE_SKIP_CHECK:-}" != "1" ]]; then
  set +e
  timeout 600 "$OODAC_BIN" check "$MAIN_ABS" >"$TMP/main_check.out" 2>"$TMP/main_check.err"
  main_ck=$?
  set -e
  if [[ $main_ck -ne 0 ]]; then
    echo "ERR_CHECK $MAIN_ABS" >&2
    cat "$TMP/main_check.err" "$TMP/main_check.out" >&2 || true
    exit 1
  fi
fi

# Collect modules (DFS, cycle/missing fail-closed). Order: deps first, main last.
MODS=()
declare -A SEEN=()
declare -A STACK=()
collect() {
  local path="$1"
  local abs; echo "collecting $path" >&2
  if [[ "$path" = /* ]]; then abs="$path"
  else abs="$(cd "$(dirname "$path")" && pwd)/$(basename "$path")"
  fi
  if [[ -n "${STACK[$abs]:-}" ]]; then echo "ERR_IMPORT_CYCLE $abs" >&2; exit 1; fi
  if [[ -n "${SEEN[$abs]:-}" ]]; then return 0; fi
  if [[ ! -f "$abs" ]]; then
    local fname="$(basename "$abs")"
    if [[ -n "${OODA_STD:-}" && -f "${OODA_STD}/$fname" ]]; then abs="${OODA_STD}/$fname"
    elif [[ -f "/home/jeryd/Projects/openOODA/std/$fname" ]]; then abs="/home/jeryd/Projects/openOODA/std/$fname"
    elif [[ -f "std/$fname" ]]; then abs="$(pwd)/std/$fname"
    elif [[ -f "../std/$fname" ]]; then abs="$(pwd)/../std/$fname"
    else echo "ERR_MISSING $abs" >&2; exit 1; fi
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

echo "pure_build: generating c files"

# 1. Preamble (only once, from main)
EMIT_NO_CONCAT=1 "$OODAC_BIN" emit-preamble "$MAIN" > "$TMP/all.c"

# 2. Protos (all files)
for src in "${MODS[@]}"; do
  EMIT_NO_CONCAT=1 timeout 600 "$OODAC_BIN" emit-protos "$src" >> "$TMP/all.c" 2>>"$TMP/emit.err" || { echo "ERR_EMIT_PROTOS $src"; cat "$TMP/emit.err"; exit 1; }
done

# 3. Bodies (all files)
for src in "${MODS[@]}"; do
  echo "emitting bodies for $src" >&2
  EMIT_NO_CONCAT=1 "$OODAC_BIN" emit-bodies "$src" >> "$TMP/all.c"
done

if ! grep -q 'int main\|long long main' "$TMP/all.c" && ! grep -q 'main(int argc' "$TMP/all.c"; then
  echo "int main(void) { return 0; }" >> "$TMP/all.c"
fi

gcc -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/all.c" -lm -o "$OUT"
test -x "$OUT"
echo OK_PURE_MULTI
rm -rf "$TMP"
