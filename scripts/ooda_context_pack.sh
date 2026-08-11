#!/usr/bin/env bash
# job: token-minimal file/symbol context pack (outline ≫ full source)
# in:  <file.oo> [symbol]
# out: outline always; reflect if available; optional symbol slice (max 40 lines)
# never prints whole files over 80 lines without --force-source
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
FORCE_SRC=0
FILE=""
SYM=""
for a in "$@"; do
  case "$a" in
    --force-source) FORCE_SRC=1 ;;
    *)
      if [[ -z "$FILE" ]]; then FILE="$a"
      elif [[ -z "$SYM" ]]; then SYM="$a"
      fi
      ;;
  esac
done
if [[ -z "$FILE" ]]; then
  echo "usage: ooda_context_pack.sh <file.oo> [symbol] [--force-source]" >&2
  exit 2
fi
if [[ ! -f "$FILE" && -f "$ROOT/$FILE" ]]; then FILE="$ROOT/$FILE"; fi
if [[ ! -f "$FILE" ]]; then echo "ERR missing $FILE" >&2; exit 1; fi

bytes=$(wc -c <"$FILE" | tr -d ' ')
lines=$(wc -l <"$FILE" | tr -d ' ')
# rough tokens ~ chars/4
est=$(( (bytes + 3) / 4 ))
echo "# context_pack $FILE lines=$lines bytes=$bytes ~tokens=$est"
echo

echo "## outline"
if [[ -x "$OODAC_BIN" ]]; then
  timeout 20 "$OODAC_BIN" outline "$FILE" 2>/dev/null || echo "(outline failed)"
else
  # fallback: pub fn / type headers only
  grep -nE '^(pub )?(fn|type|struct) ' "$FILE" | head -40 || true
fi
echo

echo "## reflect"
if [[ -x "$OODAC_BIN" ]]; then
  timeout 20 "$OODAC_BIN" reflect "$FILE" ${SYM:+"$SYM"} 2>/dev/null | head -80 || echo "(reflect failed)"
else
  echo "(no oodac)"
fi
echo

if [[ -n "$SYM" ]]; then
  echo "## symbol_slice $SYM (max 40 lines)"
  python3 - "$FILE" "$SYM" <<'PY'
import sys, re
path, sym = sys.argv[1], sys.argv[2]
lines = open(path).read().splitlines()
pat = re.compile(rf'^(pub\s+)?(fn|type|struct|let)\s+{re.escape(sym)}\b')
start = None
for i, ln in enumerate(lines):
    if pat.search(ln.strip()) or re.search(rf'\bfn\s+{re.escape(sym)}\b', ln):
        start = i
        break
if start is None:
    print(f"(symbol {sym} not found)")
    sys.exit(0)
# brace slice
depth = 0
seen = False
out = []
for j in range(start, min(len(lines), start + 80)):
    ln = lines[j]
    out.append(f"{j+1}:{ln}")
    depth += ln.count("{") - ln.count("}")
    if "{" in ln:
        seen = True
    if seen and depth <= 0:
        break
    if len(out) >= 40:
        out.append(f"... truncated at 40 lines (use --force-source for full file)")
        break
print("\n".join(out))
PY
  echo
fi

if [[ "$FORCE_SRC" == "1" ]]; then
  echo "## full_source (forced)"
  cat "$FILE"
elif [[ "$lines" -le 80 ]]; then
  echo "## full_source (file ≤80 lines)"
  cat "$FILE"
else
  echo "## full_source skipped (lines=$lines ~tokens=$est) — use outline/reflect/symbol or --force-source"
fi
