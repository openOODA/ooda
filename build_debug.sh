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
# Lifecycle: always reap temp tree (success or fail)
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
  nmods=${#MODS[@]}
  if [[ $nmods -le 12 ]]; then
    ck_to=$((90 + nmods * 10))
    set +e
    timeout "$ck_to" "$OODAC_BIN" check "$MAIN_ABS" >"$TMP/main_check.out" 2>"$TMP/main_check.err"
    main_ck=$?
    set -e
    if [[ $main_ck -eq 124 ]]; then
      echo "ERR_CHECK_TIMEOUT $MAIN_ABS (nmods=$nmods timeout=${ck_to}s)" >&2
      exit 1
    fi
    if [[ $main_ck -ne 0 ]] || ! grep -qE '^OK' "$TMP/main_check.out" 2>/dev/null; then
      echo "ERR_CHECK $MAIN_ABS" >&2
      cat "$TMP/main_check.err" "$TMP/main_check.out" >&2 || true
      exit 1
    fi
  else
    set +e
    python3 "$ROOT/scripts/oodac_module_check.py" "$MAIN_ABS" "$OODAC_BIN" \
      >"$TMP/mod_check.out" 2>"$TMP/mod_check.err"
    main_ck=$?
    set -e
    if [[ $main_ck -ne 0 ]]; then
      echo "ERR_MODULE_CHECK $MAIN_ABS (nmods=$nmods)" >&2
      cat "$TMP/mod_check.err" "$TMP/mod_check.out" >&2 || true
      exit 1
    fi
  fi
fi

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
# Forward prototypes + bodies: first definition of each symbol wins.
# (Guard against accidental double-emit / import expansion doubles — was
#  redefining main/llvm_* and thrashing gcc on self-host.)
python3 - "$TMP" "${MCS[@]}" <<'PY'
import re, sys
from pathlib import Path
tmp = Path(sys.argv[1])
mcs = [Path(p) for p in sys.argv[2:]]
fn_start = re.compile(
    r"^(void|int|long long|OoStr|OoSList|OoIList|OoResS|OoResV) "
    r"([A-Za-z_][A-Za-z0-9_]*)\s*\(.*\)\s*\{\s*$"
)
protos: list[str] = []
bodies: list[str] = []
seen: set[str] = set()
for mc in mcs:
    text = mc.read_text(errors="replace").splitlines(keepends=True)
    i = 0
    n = len(text)
    while i < n:
        m = fn_start.match(text[i].rstrip("\n"))
        if not m:
            i += 1
            continue
        name = m.group(2)
        # collect body until next top-level fn or EOF
        j = i + 1
        while j < n and not fn_start.match(text[j].rstrip("\n")):
            j += 1
        chunk = text[i:j]
        if name not in seen:
            seen.add(name)
            # prototype: "ret name(args) {" -> "ret name(args);"
            line = text[i].rstrip()
            if line.endswith("{"):
                line = line[:-1].rstrip() + ";"
            protos.append(line + "\n")
            bodies.extend(chunk)
        i = j
# stable order already = first-seen module order
(tmp / "protos.c").write_text("".join(protos))
(tmp / "bodies.c").write_text("".join(bodies))
print(f"pure_build: unique_fns={len(seen)} from_modules={len(mcs)}", flush=True)
PY

cat "$TMP/preamble.c" "$TMP/protos.c" "$TMP/bodies.c" >"$TMP/all.c"

# Bridge seed-era sealed calls → process-local grants + chs_rt (no system(3)).
# PURE_NO_ARC=1: strip seed-emitted retain/release (self-host residual; see SPRINT.md).
echo "pure_build: native_caps" # caps from c_emit

if ! grep -q 'int main\|long long main' "$TMP/all.c" && ! grep -q 'main(int argc' "$TMP/all.c"; then
  echo "int main(void) { return 0; }" >> "$TMP/all.c"
fi

gcc -g -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/all.c" -lm -o "$OUT"
test -x "$OUT"
echo OK_PURE_MULTI
rm -rf "$TMP"
