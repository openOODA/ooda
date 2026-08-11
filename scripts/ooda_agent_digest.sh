#!/usr/bin/env bash
# job: single-shot token-minimal orient pack for openOODA agent sessions
# in:  optional --emit-sample N (default 0 = skip emit matrix)
# out: tip binary, git tip, residual slice, smoke pointers (~1–3k tokens max)
# never dumps pure all.c, full SPRINT, or emit C bodies
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
EMIT_SAMPLE=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --emit-sample) EMIT_SAMPLE="${2:-12}"; shift 2 ;;
    --emit-all) EMIT_SAMPLE=9999; shift ;;
    *) shift ;;
  esac
done

echo "# openOODA agent digest"
echo "generated $(date -Iseconds 2>/dev/null || date)"
echo

echo "## tip"
if [[ -x "$OODAC_BIN" ]]; then
  ls -la "$OODAC_BIN" | awk '{print $5, $6, $7, $8, $9}'
  file "$OODAC_BIN" 2>/dev/null | head -1 || true
else
  echo "MISSING OODAC_BIN=$OODAC_BIN"
fi
echo "git $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null) $(git -C "$ROOT" log -1 --oneline 2>/dev/null | cut -d' ' -f2-)"
echo "branch $(git -C "$ROOT" status -sb 2>/dev/null | head -1)"
echo

echo "## do_not_read (token traps)"
cat <<'EOF'
- ~/.cache/ooda-tmp/**/all*.c  (~700KB pure emit)
- openOODA/SPRINT.md full dump (use residual slice below)
- full gcc logs (use scripts/ooda_err_digest.sh)
- full emit-c stdout (use outline/reflect or emit_health)
- mass-annotate entire oodac/*.oo without a SEGV repro first
EOF
echo

echo "## prefer (token-cheap orient)"
cat <<'EOF'
- oodac outline <file.oo>          # signatures only (~1% of source)
- oodac reflect <file.oo>          # JSON caps+sig
- scripts/ooda_product_context.sh <file> <symbol>
- scripts/ooda_emit_health.sh [mods…]
- scripts/ooda_err_digest.sh <log>
- scripts/ooda_agent_digest.sh     # this pack
EOF
echo

echo "## residual slice (latest M17x only — not full SPRINT)"
SPRINT_CANDIDATES=(
  "$ROOT/../openOODA/SPRINT.md"
  "$ROOT/../../openOODA/openOODA/SPRINT.md"
  "/home/jeryd/Projects/openOODA/openOODA/SPRINT.md"
)
SPRINT=""
for c in "${SPRINT_CANDIDATES[@]}"; do
  [[ -f "$c" ]] && SPRINT="$c" && break
done
if [[ -n "$SPRINT" ]]; then
  echo "source: $SPRINT ($(wc -l <"$SPRINT") lines) — DO NOT cat whole file"
  # headers index only (line numbers)
  echo "headers:"
  grep -nE '^## M1[6-9]|^## M17' "$SPRINT" 2>/dev/null | tail -12 || true
  # latest ## M17x block only (max 35 lines)
  python3 - "$SPRINT" <<'PY'
import sys, re
path = sys.argv[1]
lines = open(path).read().splitlines()
starts = [i for i,l in enumerate(lines) if re.match(r'^## M17\d', l)]
if not starts:
    starts = [i for i,l in enumerate(lines) if re.match(r'^## M16\d', l)]
if not starts:
    print("(no M16x/M17x section)")
    sys.exit(0)
i = starts[-1]
print(f"--- slice from line {i+1} ---")
for j in range(i, min(len(lines), i+35)):
    print(lines[j])
if i+35 < len(lines):
    print("… truncated (open SPRINT only if needed)")
PY
else
  echo "(no SPRINT.md found)"
fi
echo

echo "## product smoke pointers (run, don't paste logs)"
echo "scripts: agy_lang_blockers m169_residual_closeout cap_forge_path_a malloc_path_a bitwise_ops"
echo "rule: capture exit code + last line only"
echo

if [[ "$EMIT_SAMPLE" != "0" ]]; then
  echo "## emit_health sample"
  if [[ "$EMIT_SAMPLE" == "9999" ]]; then
    bash "$ROOT/scripts/ooda_emit_health.sh" 2>/dev/null | tail -40 || true
  else
    # sample high-risk + a few green
    bash "$ROOT/scripts/ooda_emit_health.sh" \
      check_caps check_drive c_emit_fn c_emit_let main lex token_scan_punct \
      2>/dev/null || true
  fi
  echo
fi

echo "## leave_off template (fill; keep under 15 lines)"
cat <<'EOF'
tip: oodac/oodac (=m??? )
green: …
residual: …
next: …
do_not: paste all_fix*.c / full SPRINT / 500-line gcc logs
EOF
