#!/usr/bin/env bash
# Lock: owned source files must be ≤ MAX_LINES (default 256).
# See bootstrap/SPLIT_PLAN.md and TOOLS.md (entropy O).
#
# Usage:
#   ./scripts/check_file_lines.sh           # list violators; exit 1 if any
#   ./scripts/check_file_lines.sh --ratchet # exit 1 only if oversize grew / new oversize
#   ./scripts/check_file_lines.sh --json    # machine-readable summary on stdout
#
# Owned source: .oo .rs .c .h .sh .py under repo root, excluding generated/noise.
set -euo pipefail

if git rev-parse --show-toplevel >/dev/null 2>&1; then
  ROOT="$(git rev-parse --show-toplevel)"
else
  ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
fi
cd "$ROOT"

MAX_LINES="${MAX_LINES:-256}"
MODE="strict"
JSON=0
for arg in "$@"; do
  case "$arg" in
    --ratchet) MODE="ratchet" ;;
    --json) JSON=1 ;;
    --help|-h)
      sed -n '2,12p' "$0"
      exit 0
      ;;
  esac
done

# Paths that are emit dumps / build artifacts — not hand-owned source.
is_excluded() {
  local f="$1"
  case "$f" in
    *'/.git/'*|*'target/'*|*'dist/'*|*'/.agents/'*) return 0 ;;
    *.oo.c|*.oo.bin|*.c_native|*.concat.oo|*.oo.concat.oo) return 0 ;;
    oodac/main.c|oodac/oodac2.c|./oodac/main.c|./oodac/oodac2.c) return 0 ;;
    */oodac/main.c|*/oodac/oodac2.c) return 0 ;;
    a.out|*/a.out|out.txt|*/out.txt) return 0 ;;
  esac
  return 1
}

# Collect: relpath lines
mapfile -t ALL < <(
  find . -type f \
    \( -name '*.oo' -o -name '*.rs' -o -name '*.c' -o -name '*.h' -o -name '*.sh' -o -name '*.py' \) \
    -not -path './.git/*' \
    -not -path './target/*' \
    -not -path './dist/*' \
    2>/dev/null | sed 's|^\./||' | sort
)

declare -a VIOL_PATHS=()
declare -a VIOL_LINES=()
O=0
TOTAL=0

for f in "${ALL[@]}"; do
  is_excluded "$f" && continue
  [[ -f "$f" ]] || continue
  TOTAL=$((TOTAL + 1))
  n=$(wc -l < "$f" | tr -d ' ')
  if (( n > MAX_LINES )); then
    VIOL_PATHS+=("$f")
    VIOL_LINES+=("$n")
    O=$((O + 1))
  fi
done

if (( JSON )); then
  echo "{"
  echo "  \"max_lines\": $MAX_LINES,"
  echo "  \"owned_files\": $TOTAL,"
  echo "  \"O\": $O,"
  echo "  \"violators\": ["
  for i in "${!VIOL_PATHS[@]}"; do
    comma=","; (( i == ${#VIOL_PATHS[@]} - 1 )) && comma=""
    printf '    {"path": "%s", "lines": %s}%s\n' "${VIOL_PATHS[$i]}" "${VIOL_LINES[$i]}" "$comma"
  done
  echo "  ]"
  echo "}"
else
  echo "check_file_lines: MAX_LINES=$MAX_LINES  owned=$TOTAL  O=$O  mode=$MODE"
  if (( O > 0 )); then
    echo "OVERSIZE (lines > $MAX_LINES):"
    for i in "${!VIOL_PATHS[@]}"; do
      printf '  %6s  %s\n' "${VIOL_LINES[$i]}" "${VIOL_PATHS[$i]}"
    done | sort -n
  else
    echo "OK: no owned source file exceeds $MAX_LINES lines."
  fi
fi

# Ratchet: fail only if an oversize file grew vs HEAD, or a new oversize file appears.
if [[ "$MODE" == "ratchet" ]]; then
  if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "ratchet: not a git repo; falling back to strict" >&2
    MODE="strict"
  fi
fi

if [[ "$MODE" == "ratchet" ]]; then
  FAIL=0
  for i in "${!VIOL_PATHS[@]}"; do
    f="${VIOL_PATHS[$i]}"
    n="${VIOL_LINES[$i]}"
    if git cat-file -e "HEAD:$f" 2>/dev/null; then
      old=$(git show "HEAD:$f" 2>/dev/null | wc -l | tr -d ' ')
      if (( n > old )); then
        echo "RATCHET FAIL: $f grew $old → $n (still over $MAX_LINES)" >&2
        FAIL=1
      fi
    else
      echo "RATCHET FAIL: new oversize file $f ($n lines)" >&2
      FAIL=1
    fi
  done
  if (( FAIL )); then
    exit 1
  fi
  if (( O > 0 )); then
    echo "ratchet OK: oversize files present (O=$O) but none grew; split still required for strict Lock."
  fi
  exit 0
fi

# Strict
if (( O > 0 )); then
  echo "STRICT FAIL: O=$O owned source file(s) over $MAX_LINES lines. See bootstrap/SPLIT_PLAN.md" >&2
  exit 1
fi
exit 0
