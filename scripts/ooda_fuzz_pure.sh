#!/usr/bin/env bash
# job: pure contract fuzz harness (Int + Bool + String + List; no Python)
# in:  OODA_TEST_SRC, OODA_TEST_HARNESS, OODA_TEST_FUZZ_ITERS, OODA_TEST_FUZZ_SEED
#      optional OODA_TEST_FUZZ_VERBOSE=1
# out: writes harness .oo; exit 0 on success; exit 2 fail-closed unsupported
# markers (source):
#   // FUZZ_DOMAIN: int | bool | string | list  (alias list_int → list)
#   // FUZZ_TARGET: <fn> <min> <max>   # int value | string/list length
#   // FUZZ_TARGET: <fn>                 # bool only
#   // FUZZ_REQUIRES: <fn> <expr with x>
#   // FUZZ_ENSURES: <fn> <expr with x and/or result>
set -euo pipefail
ROOT_FUZZ="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=ooda_fuzz_pure_gens.sh
source "$ROOT_FUZZ/ooda_fuzz_pure_gens.sh"
SRC="${OODA_TEST_SRC:?OODA_TEST_SRC}"
OUT="${OODA_TEST_HARNESS:?OODA_TEST_HARNESS}"
ITERS="${OODA_TEST_FUZZ_ITERS:-100}"
SEED="${OODA_TEST_FUZZ_SEED:-42}"
VERBOSE="${OODA_TEST_FUZZ_VERBOSE:-0}"
if [[ ! -f "$SRC" ]]; then
  echo "ERR	fuzz	unreadable file: $SRC" >&2
  exit 2
fi
if ! [[ "$ITERS" =~ ^[0-9]+$ ]] || [[ "$ITERS" -le 0 ]]; then
  echo "ERR	fuzz	invalid iterations: $ITERS" >&2
  exit 2
fi
if ! [[ "$SEED" =~ ^-?[0-9]+$ ]]; then
  echo "ERR	fuzz	invalid seed: $SEED" >&2
  exit 2
fi
# Domain gate: int | bool | string | list (fail-closed otherwise)
DOMAIN_LINE=$(grep -E '^[[:space:]]*//[[:space:]]*FUZZ_DOMAIN:' "$SRC" | head -1 || true)
if [[ -z "$DOMAIN_LINE" ]]; then
  echo "ERR	fuzz	pure path supports only // FUZZ_DOMAIN: int|bool|string|list fixtures (fail-closed)" >&2
  exit 2
fi
DOMAIN="${DOMAIN_LINE#*FUZZ_DOMAIN:}"
DOMAIN="${DOMAIN#"${DOMAIN%%[![:space:]]*}"}"
DOMAIN="${DOMAIN%%[[:space:]]*}"
if [[ "$DOMAIN" == "list_int" ]]; then DOMAIN="list"; fi
if [[ "$DOMAIN" != "int" && "$DOMAIN" != "bool" && "$DOMAIN" != "string" && "$DOMAIN" != "list" ]]; then
  echo "ERR	fuzz	pure path supports only // FUZZ_DOMAIN: int|bool|string|list fixtures (fail-closed)" >&2
  exit 2
fi
mapfile -t TARGET_LINES < <(grep -E '^[[:space:]]*//[[:space:]]*FUZZ_TARGET:' "$SRC" || true)
if [[ ${#TARGET_LINES[@]} -eq 0 ]]; then
  echo "ERR	fuzz	FUZZ_DOMAIN $DOMAIN but no // FUZZ_TARGET" >&2
  exit 2
fi
TARGETS=()
MINS=()
MAXS=()
for line in "${TARGET_LINES[@]}"; do
  rest="${line#*FUZZ_TARGET:}"
  rest="${rest#"${rest%%[![:space:]]*}"}"
  # shellcheck disable=SC2086
  set -- $rest
  if [[ "$DOMAIN" == "int" || "$DOMAIN" == "string" || "$DOMAIN" == "list" ]]; then
    if [[ $# -lt 3 ]]; then
      echo "ERR	fuzz	bad FUZZ_TARGET (need name min max): $line" >&2
      exit 2
    fi
    tname="$1"
    tmin="$2"
    tmax="$3"
    if ! [[ "$tmin" =~ ^-?[0-9]+$ && "$tmax" =~ ^-?[0-9]+$ ]]; then
      echo "ERR	fuzz	FUZZ_TARGET min/max must be ints: $line" >&2
      exit 2
    fi
    if [[ "$tmin" -gt "$tmax" ]]; then
      echo "ERR	fuzz	FUZZ_TARGET min > max: $line" >&2
      exit 2
    fi
    TARGETS+=("$tname")
    MINS+=("$tmin")
    MAXS+=("$tmax")
  else
    # bool: name only
    if [[ $# -lt 1 ]]; then
      echo "ERR	fuzz	bad FUZZ_TARGET (need name): $line" >&2
      exit 2
    fi
    TARGETS+=("$1")
    MINS+=("0")
    MAXS+=("1")
  fi
done
# requires/ensures keyed by fn name (last wins if multiple)
declare -A REQ_EXPR=()
declare -A ENS_EXPR=()
while IFS= read -r line; do
  rest="${line#*FUZZ_REQUIRES:}"
  rest="${rest#"${rest%%[![:space:]]*}"}"
  # shellcheck disable=SC2086
  set -- $rest
  [[ $# -lt 2 ]] && continue
  fn="$1"
  shift
  REQ_EXPR["$fn"]="$*"
done < <(grep -E '^[[:space:]]*//[[:space:]]*FUZZ_REQUIRES:' "$SRC" || true)
while IFS= read -r line; do
  rest="${line#*FUZZ_ENSURES:}"
  rest="${rest#"${rest%%[![:space:]]*}"}"
  # shellcheck disable=SC2086
  set -- $rest
  [[ $# -lt 2 ]] && continue
  fn="$1"
  shift
  # AND multi-// FUZZ_ENSURES for same fn (not last-wins)
  if [[ -n "${ENS_EXPR[$fn]+x}" && -n "${ENS_EXPR[$fn]}" ]]; then
    ENS_EXPR["$fn"]="(${ENS_EXPR[$fn]}) && ($*)"
  else
    ENS_EXPR["$fn"]="$*"
  fi
done < <(grep -E '^[[:space:]]*//[[:space:]]*FUZZ_ENSURES:' "$SRC" || true)
# Emit body functions: strip markers, requires/ensures, and main
body_tmp="$(mktemp "${TMPDIR:-/tmp}/ooda_fuzz_body.XXXXXX")"
cleanup_body() { rm -f "$body_tmp"; }
trap cleanup_body EXIT
awk '
  function brace_delta(s,   i, c, d) {
    d = 0
    for (i = 1; i <= length(s); i++) {
      c = substr(s, i, 1)
      if (c == "{") d++
      if (c == "}") d--
    }
    return d
  }
  BEGIN { in_main = 0; depth = 0; main_seen_brace = 0 }
  /^[[:space:]]*\/\/[[:space:]]*FUZZ_/ { next }
  /^[[:space:]]*requires[[:space:]]/ { next }
  /^[[:space:]]*ensures[[:space:]]/ { next }
  !in_main && /^(pub[[:space:]]+)?fn[[:space:]]+main[[:space:]]*\(/ {
    in_main = 1
    depth = 0
    main_seen_brace = 0
  }
  in_main {
    d = brace_delta($0)
    if (d != 0 || index($0, "{") || index($0, "}")) {
      main_seen_brace = 1
      depth += d
      if (main_seen_brace && depth <= 0) {
        in_main = 0
        depth = 0
        main_seen_brace = 0
      }
    }
    next
  }
  { print }
' "$SRC" >"$body_tmp"

if ! grep -qE '^(pub[[:space:]]+)?fn[[:space:]]+' "$body_tmp"; then
  echo "ERR	fuzz	no non-main functions to fuzz in $SRC" >&2
  exit 2
fi

{
  echo "// generated by ooda_fuzz_pure.sh (pure ${DOMAIN} domain; no Python)"
  echo ""
  cat "$body_tmp"
  echo ""
  emit_fuzz_generators
  echo ""
  echo "pub fn main() {"
  echo "    println(\"openOODA Fuzzer pure ${DOMAIN} domain running...\");"
  echo "    let mut __fuzz_prng_st = ${SEED};"
  echo "    let mut __fuzz_total_filtered = 0;"

  ti=0
  for fname in "${TARGETS[@]}"; do
    tmin="${MINS[$ti]}"
    tmax="${MAXS[$ti]}"
    req="${REQ_EXPR[$fname]:-}"
    ens="${ENS_EXPR[$fname]:-}"
    req_chk=""
    ens_chk=""
    if [[ -n "$req" ]]; then
      req_chk="$(echo "$req" | sed 's/\bx\b/__fuzz_x/g')"
    fi
    if [[ -n "$ens" ]]; then
      ens_chk="$(echo "$ens" | sed 's/\bresult\b/__fuzz_r/g; s/\bx\b/__fuzz_x/g')"
      # List[Int] == does not lower on Backend-C — rewrite to pure list_eq_int.
      if [[ "$DOMAIN" == "list" ]]; then
        ens_chk="$(echo "$ens_chk" | sed \
          -e 's/__fuzz_r[[:space:]]*==[[:space:]]*__fuzz_x/list_eq_int(__fuzz_r, __fuzz_x)/g' \
          -e 's/__fuzz_x[[:space:]]*==[[:space:]]*__fuzz_r/list_eq_int(__fuzz_x, __fuzz_r)/g')"
      fi
    fi

    echo "    let mut __fuzz_comp_${fname} = 0;"
    echo "    let mut __fuzz_cons_filt_${fname} = 0;"
    echo "    while __fuzz_comp_${fname} < ${ITERS} {"
    echo "        __fuzz_prng_st = prng_step(__fuzz_prng_st);"
    emit_fuzz_sample_let "        "

    if [[ -n "$req_chk" ]]; then
      echo "        let __fuzz_req_ok: Bool = (${req_chk});"
      echo "        if !__fuzz_req_ok {"
      echo "            __fuzz_total_filtered = __fuzz_total_filtered + 1;"
      echo "            __fuzz_cons_filt_${fname} = __fuzz_cons_filt_${fname} + 1;"
      echo "            if __fuzz_cons_filt_${fname} >= 2000 {"
      echo "                println(\"ERR: could not satisfy precondition for function '${fname}'\");"
      echo "                println(\"FUZZ_FAIL: could not satisfy precondition\");"
      echo "                process_exit(1);"
      echo "            }"
      echo "        } else {"
      echo "            __fuzz_cons_filt_${fname} = 0;"
      echo "            __fuzz_comp_${fname} = __fuzz_comp_${fname} + 1;"
      emit_fuzz_call_let "            "
      if [[ -n "$ens_chk" ]]; then
        echo "            let __fuzz_ens_ok: Bool = (${ens_chk});"
        echo "            if !__fuzz_ens_ok {"
        echo "                println(\"FUZZ_FAIL: postcondition failed\");"
        echo "                println(\"Function: ${fname}\");"
        echo "                process_exit(1);"
        echo "            }"
      fi
      if [[ "$VERBOSE" == "1" ]]; then
        echo "            println(\"PASS ${fname}\");"
      fi
      echo "        }"
    else
      echo "        __fuzz_comp_${fname} = __fuzz_comp_${fname} + 1;"
      emit_fuzz_call_let "        "
      if [[ -n "$ens_chk" ]]; then
        echo "        let __fuzz_ens_ok: Bool = (${ens_chk});"
        echo "        if !__fuzz_ens_ok {"
        echo "            println(\"FUZZ_FAIL: postcondition failed\");"
        echo "            println(\"Function: ${fname}\");"
        echo "            process_exit(1);"
        echo "        }"
      fi
      if [[ "$VERBOSE" == "1" ]]; then
        echo "        println(\"PASS ${fname}\");"
      fi
    fi
    echo "    }"
    ti=$((ti + 1))
  done

  echo "    println(\"openOODA Fuzzer pure ${DOMAIN} domain passed\");"
  echo "}"
} >"$OUT"

echo "OK	fuzz	pure ${DOMAIN} harness ${#TARGETS[@]} target(s) iters=${ITERS} seed=${SEED}" >&2
exit 0
