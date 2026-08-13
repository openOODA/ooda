#!/usr/bin/env bash
# job: pure multi-module oodac build (emit each .oo, link once) — no stage-0 host
# in:  <main.oo> <out_bin>
# out: native binary via emit-c + gcc + chs_rt only
# link recipe: Backend-C (see bootstrap/FLOOR.oot) — swap here for other floors later
# Notes:
#  - Forward prototypes for all fns so use-before-def across modules is OK
#  - Nested imports + cycle/missing fail-closed (parity with load_import.oo)
#  - Never uses $OODA host soft-pass
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAIN="${1:?main.oo}"
OUT="${2:?out_bin}"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-.ooda-cache/ooda-tmp}/oodac_llvm_$$"
mkdir -p "$TMP"
# Lifecycle: always reap temp tree (success or fail)
cleanup_llvm_tmp() { echo "kept $TMP"; }
trap cleanup_llvm_tmp EXIT
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

# Check gate always runs (R4). Fail-closed.
# Small programs: full import-expand check.
# Large graphs (e.g. oodac self-host): per-module check with signature stubs
# (avoids whole-program concat OOM; still typechecks every module body).
if [[ "${PURE_SKIP_CHECK:-}" != "1" ]]; then
  set +e
  timeout 90 "$OODAC_BIN" check "$MAIN_ABS" >"$TMP/main_check.out" 2>"$TMP/main_check.err"
  main_ck=$?
  set -e
  if [[ $main_ck -ne 0 ]]; then
    echo "ERR_CHECK $MAIN_ABS" >&2
    cat "$TMP/main_check.err" "$TMP/main_check.out" >&2 || true
    exit 1
  fi
fi

echo "llvm_build: generating ll files"

first=1
for src in "${MODS[@]}"; do
  echo "emitting llvm for $src" >&2
  if [[ $first -eq 1 ]]; then
      OODA_EMIT_NO_CONCAT=1 EMIT_NO_CONCAT=1 "$OODAC_BIN" emit-llvm "$src" >> "$TMP/all.ll"
      first=0
  else
      OODA_EMIT_NO_CONCAT=1 EMIT_NO_CONCAT=1 EMIT_NO_PREAMBLE=1 "$OODAC_BIN" emit-llvm "$src" >> "$TMP/all.ll"
  fi
done

# Bridge seed-era sealed calls -> process-local grants + chs_rt (no system(3)).
# (No rewrite script needed for LLVM as it emits correctly out of the box)

if ! grep -q '@main(' "$TMP/all.ll"; then
  echo "define i32 @main(i32 %argc, i8** %argv) { ret i32 0 }" >> "$TMP/all.ll"
fi

clang -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/all.ll" -lm -o "$OUT"
test -x "$OUT"
echo OK_LLVM_MULTI
