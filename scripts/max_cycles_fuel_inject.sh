#!/usr/bin/env bash
# job: M48 MaxCycles path A — inject Backend-C while fuel from // MAX_CYCLES: N
# in:  <src.oo> <in.c> <out.c>
# out: out.c with while-body fuel when marker present; non-zero on zero/invalid N
# residual: for-loops not fueled; not OS cgroup; #[MaxCycles] name residual
set -euo pipefail

[[ $# -eq 3 ]] || { echo "usage: max_cycles_fuel_inject.sh <src.oo> <in.c> <out.c>" >&2; exit 2; }
SRC="$1"
INC="$2"
OUTC="$3"

[[ -f "$SRC" ]] || { echo -e "ERR\tmax_cycles\tmissing source: $SRC" >&2; exit 1; }
[[ -f "$INC" ]] || { echo -e "ERR\tmax_cycles\tmissing C: $INC" >&2; exit 1; }

MARK_LINE="$(grep -E '//[[:space:]]*MAX_CYCLES:[[:space:]]*' "$SRC" 2>/dev/null | head -1 || true)"
if [[ -z "$MARK_LINE" ]]; then
  cp -- "$INC" "$OUTC"
  exit 0
fi

N_RAW="$(printf '%s\n' "$MARK_LINE" | sed -nE 's/.*MAX_CYCLES:[[:space:]]*([0-9]+).*/\1/p' | head -1)"
if [[ -z "$N_RAW" ]]; then
  echo -e "ERR\tmax_cycles\tinvalid N (need // MAX_CYCLES: <positive int>)" >&2
  exit 1
fi
N=$((10#$N_RAW))
if [[ "$N" -le 0 ]]; then
  # Attack: silent zero N — refuse; do not disable fuel
  echo -e "ERR\tmax_cycles\tzero N fail-closed (need N>0)" >&2
  exit 1
fi

# Count user whiles in input C (exclude while(0) assert macros)
USER_WHILE="$(
  grep -E 'while[[:space:]]*\(' "$INC" 2>/dev/null \
    | grep -vE 'while[[:space:]]*\([[:space:]]*0[[:space:]]*\)' \
    | wc -l | tr -d ' '
)"

awk -v N="$N" '
  BEGIN { decl=0; pend=0; fueled=0; after_main_sig=0 }
  function user_while(s) {
    return (s ~ /while[[:space:]]*\(/) && (s !~ /while[[:space:]]*\([[:space:]]*0[[:space:]]*\)/)
  }
  {
    # Fuel decl: first line of main body
    if (!decl && $0 ~ /^int main[[:space:]]*\(/) { after_main_sig=1; print; next }
    if (!decl && after_main_sig && $0 ~ /\{/) {
      print
      print "  long long __oo_mc_fuel = " N "LL; /* MAX_CYCLES path A */"
      decl=1
      after_main_sig=0
      next
    }
    if (after_main_sig && $0 !~ /^[[:space:]]*$/) after_main_sig=0

    if (user_while($0)) {
      if (!decl) {
        print "static long long __oo_mc_fuel = " N "LL; /* MAX_CYCLES path A */"
        decl=1
      }
      print
      if ($0 ~ /\{[[:space:]]*$/) {
        print "  if (__oo_mc_fuel-- <= 0LL) { fprintf(stderr, \"ERR\\tmax_cycles\\texceeded\\n\"); exit(1); }"
        fueled++
      } else {
        pend=1
      }
      next
    }
    if (pend) {
      print
      if ($0 ~ /\{/) {
        print "  if (__oo_mc_fuel-- <= 0LL) { fprintf(stderr, \"ERR\\tmax_cycles\\texceeded\\n\"); exit(1); }"
        fueled++
        pend=0
      }
      next
    }
    print
  }
  END {
    if (!decl) {
      # No main / no while: still emit marker honesty via exit status from shell
    }
  }
' "$INC" >"${OUTC}.tmp"

# Marker + no fuel decl → inject static at top (no while case)
if ! grep -q '__oo_mc_fuel' "${OUTC}.tmp"; then
  {
    echo "/* MAX_CYCLES path A: N=$N (no while fueled in TU) */"
    echo "static long long __oo_mc_fuel = ${N}LL;"
    cat "$INC"
  } >"${OUTC}.tmp"
fi

# Attack: fuel not applied to user whiles when they exist
if [[ "${USER_WHILE:-0}" -gt 0 ]]; then
  CHECKS="$(grep -c '__oo_mc_fuel--' "${OUTC}.tmp" || true)"
  if [[ "${CHECKS:-0}" -lt 1 ]]; then
    echo -e "ERR\tmax_cycles\tfuel not applied to while loops" >&2
    rm -f "${OUTC}.tmp"
    exit 1
  fi
fi

mv "${OUTC}.tmp" "$OUTC"
exit 0
