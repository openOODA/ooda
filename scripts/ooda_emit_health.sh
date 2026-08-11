#!/usr/bin/env bash
# job: compact emit-health matrix for oodac modules (token-minimal agent orient)
# in:  optional module globs or paths; env OODAC_BIN, EMIT_TIMEOUT (default 12)
# out: one line per module: name STATUS lines=N  (no C dumps)
# exit: 0 if all OK; 1 if any SEGV/ERR; 2 if any TIMEOUT (and no SEGV)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
TO="${EMIT_TIMEOUT:-12}"
if [[ ! -x "$OODAC_BIN" ]]; then
  echo "ERR no OODAC_BIN=$OODAC_BIN" >&2
  exit 1
fi

mods=()
if [[ $# -gt 0 ]]; then
  for a in "$@"; do
    if [[ -f "$a" ]]; then mods+=("$a")
    elif [[ -f "oodac/$a" ]]; then mods+=("oodac/$a")
    elif [[ -f "oodac/$a.oo" ]]; then mods+=("oodac/$a.oo")
    else echo "SKIP missing $a" >&2; fi
  done
else
  # default: oodac/*.oo sorted
  while IFS= read -r f; do mods+=("$f"); done < <(ls oodac/*.oo 2>/dev/null | sort)
fi

n_ok=0 n_segv=0 n_to=0 n_err=0 n_tot=0
declare -a bad=()
for f in "${mods[@]}"; do
  n_tot=$((n_tot + 1))
  base=$(basename "$f" .oo)
  out="/tmp/ooda_emit_health_$$_${base}.c"
  set +e
  timeout "$TO" env EMIT_NO_CONCAT=1 EMIT_NO_PREAMBLE=1 \
    "$OODAC_BIN" emit-c "$f" >"$out" 2>/tmp/ooda_emit_health_$$.err
  ec=$?
  set -e
  lines=$(wc -l <"$out" 2>/dev/null | tr -d ' ' || echo 0)
  case $ec in
    0) st=OK; n_ok=$((n_ok+1)) ;;
    124) st=TIMEOUT; n_to=$((n_to+1)); bad+=("$base:TIMEOUT") ;;
    139|134) st=SEGV; n_segv=$((n_segv+1)); bad+=("$base:SEGV") ;;
    *) st="EC$ec"; n_err=$((n_err+1)); bad+=("$base:EC$ec") ;;
  esac
  # compact: STATUS name lines=
  printf '%-8s %-28s lines=%s\n' "$st" "$base" "$lines"
  rm -f "$out"
done
rm -f /tmp/ooda_emit_health_$$.err 2>/dev/null || true

echo "---"
echo "summary ok=$n_ok segv=$n_segv timeout=$n_to err=$n_err total=$n_tot"
if [[ ${#bad[@]} -gt 0 ]]; then
  echo "bad ${bad[*]}"
fi

if [[ $n_segv -gt 0 || $n_err -gt 0 ]]; then exit 1; fi
if [[ $n_to -gt 0 ]]; then exit 2; fi
exit 0
