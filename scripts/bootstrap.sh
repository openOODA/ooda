#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAIN="${1:?main.oo}"
OUT="${2:?out_bin}"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-.ooda-cache/ooda-tmp}/oodac_pure_$$"
mkdir -p "$TMP"
cleanup_pure_tmp() { rm -rf "$TMP"; }
#trap cleanup_pure_tmp EXIT

DIR="$(cd "$(dirname "$MAIN")" && pwd)"
BASE="$(basename "$MAIN")"
MAIN_ABS="$DIR/$BASE"

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
  STACK[$abs]=1
  local dir
  dir="$(cd "$(dirname "$abs")" && pwd)"
  local imp
  while IFS= read -r imp; do
    [[ -z "$imp" ]] && continue
    if [[ "$imp" = /* ]]; then collect "$imp"
    else collect "$dir/$imp"
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
  OODA_EMIT_NO_CONCAT=1 EMIT_NO_CONCAT=1 timeout 60 "$OODAC_BIN" emit-c "$src" >"$mc" 2>"$TMP/emit.err"; ec=$?
  if [ $ec -eq 124 ]; then echo "ERR_EMIT_TIMEOUT $src" >&2; exit 1; fi
  if [ ! -s "$mc" ] || ! grep -qE "$FN_DEF" "$mc" || grep -qE "^ERR" "$mc"; then echo "ERR_EMIT $src" >&2; cat "$TMP/emit.err" >&2 || true; exit 1; fi
  MCS+=("$mc")
done

awk "/$FN_DEF/{exit} {print}" "${MCS[0]}" >"$TMP/preamble.c"

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
        j = i + 1
        while j < n and not fn_start.match(text[j].rstrip("\n")):
            j += 1
        chunk = text[i:j]
        if name not in seen:
            seen.add(name)
            line = text[i].rstrip()
            if line.endswith("{"):
                line = line[:-1].rstrip() + ";"
            protos.append(line + "\n")
            bodies.extend(chunk)
        i = j

(tmp / "protos.c").write_text("".join(protos))

# FAIRY Auto-Pagination: Shard bodies to strictly respect 256-line limit
MAX_LINES = 200
bodies_c_content = []
part = 0
lines = 0
chunk = []
for line in bodies:
    chunk.append(line)
    lines += 1
    if lines >= MAX_LINES and line.rstrip() == "}":
        (tmp / f"bodies_{part}.c").write_text("".join(chunk))
        bodies_c_content.append(f'#include "bodies_{part}.c"\n')
        part += 1
        chunk = []
        lines = 0
if chunk:
    (tmp / f"bodies_{part}.c").write_text("".join(chunk))
    bodies_c_content.append(f'#include "bodies_{part}.c"\n')

(tmp / "bodies.c").write_text("".join(bodies_c_content))
PY

cat "$TMP/preamble.c" "$TMP/protos.c" "$TMP/bodies.c" >"$TMP/all.c"

echo "pure_build: native_caps" # caps from c_emit

if ! grep -q 'int main\|long long main' "$TMP/all.c" && ! grep -q 'main(int argc' "$TMP/all.c"; then
  echo "int main(void) { return 0; }" >> "$TMP/all.c"
fi

gcc -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/all.c" -lm -ldl -lpthread -o "$OUT"
echo OK_PURE_MULTI
