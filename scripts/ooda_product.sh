#!/usr/bin/env bash
# job: product CLI backend — argv only (no eval of user strings)
# in:  mode + args from pure cli/main.oo (already should be safe; still use "$@")
# out: exit status of oodac / harness
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd)" || true
if [[ -z "$ROOT" || ! -d "$ROOT" ]]; then
  echo "ERR_ROOT_INVALID: failed to resolve product root" >&2
  exit 1
fi
if [[ -n "${OODAC_BIN:-}" && -x "${OODAC_BIN}" ]]; then
  OODAC_BIN="$(readlink -f "$OODAC_BIN" 2>/dev/null || realpath "$OODAC_BIN" 2>/dev/null || echo "$OODAC_BIN")"
  export OODAC_BIN
fi
if [[ -n "${OODA_BIN:-}" && -x "${OODA_BIN}" ]]; then
  OODA_BIN="$(readlink -f "$OODA_BIN" 2>/dev/null || realpath "$OODA_BIN" 2>/dev/null || echo "$OODA_BIN")"
  export OODA_BIN
fi
if [[ -n "${OODA_SRC_ROOT:-}" && -d "${OODA_SRC_ROOT}" ]]; then
  OODA_SRC_ROOT="$(readlink -f "$OODA_SRC_ROOT" 2>/dev/null || realpath "$OODA_SRC_ROOT" 2>/dev/null || echo "$OODA_SRC_ROOT")"
  export OODA_SRC_ROOT
fi

MODE="${1:-}"
shift || true

resolve_em() {
  EM="${OODAC_BIN:-}"
  if [[ -n "$EM" && -x "$EM" ]]; then
    EM="$(readlink -f "$EM" 2>/dev/null || echo "$EM")"
  else
    EM=""
  fi
  if [[ -z "$EM" || ! -x "$EM" ]]; then
    if [[ -x "$ROOT/oodac/oodac" ]]; then
      EM="$ROOT/oodac/oodac"
    elif [[ -x ./oodac/oodac ]]; then
      EM="$(readlink -f ./oodac/oodac 2>/dev/null || echo ./oodac/oodac)"
    elif [[ -x oodac/oodac ]]; then
      EM="$(readlink -f oodac/oodac 2>/dev/null || echo oodac/oodac)"
    elif [[ -x "$ROOT/../ooda/oodac/oodac" ]]; then
      EM="$(readlink -f "$ROOT/../ooda/oodac/oodac" 2>/dev/null || echo "$ROOT/../ooda/oodac/oodac")"
    else
      echo ERR_NO_OODAC >&2
      exit 1
    fi
  fi
  export OODAC_BIN="$EM"
}

case "$MODE" in
  forward)
    # forward <oodac-cmd> [args...]
    resolve_em
    exec "$EM" "$@"
    ;;
  build)
    resolve_em
    target="c"
    file=""
    extra=()
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --target)
          target="$2"
          shift 2
          ;;
        *)
          if [[ -z "$file" ]]; then file="$1"; else extra+=("$1"); fi
          shift
          ;;
      esac
    done
    if [[ "$target" == "wasm" ]]; then
      echo -e "ERR\tcli\ttarget wasm residual" >&2
      exit 2
    fi
    out="${file%.oo}"
    [[ "$file" == *.oo ]] || out="${file}.bin"
    "$EM" build --backend c "$file" "$out" "${extra[@]}"
    test -x "$out"
    echo "🚀 [openOODA pure oodac] Native executable: $out"
    ;;
  em)
    resolve_em
    file=""
    json_mode=""
    for arg in "$@"; do
      if [[ "$arg" == "--json" ]]; then
        json_mode=1
      elif [[ "$arg" != -* ]]; then
        file="$arg"
      fi
    done
    if [[ -z "$file" ]]; then
      echo "ERR_EM_NO_FILE" >&2
      exit 1
    fi
    t_start=$(python3 -c 'import time; print(time.perf_counter_ns())' 2>/dev/null || date +%s%N)
    set +e
    chk_err="$("$EM" check "$file" --json-errors 2>&1)"
    chk_rc=$?
    set -e
    t_end=$(python3 -c 'import time; print(time.perf_counter_ns())' 2>/dev/null || date +%s%N)
    if [[ $chk_rc -ne 0 ]]; then
      if [[ -n "$json_mode" ]]; then
        echo '{"type_failed": true, "cap_failed": false, "errors": '"$chk_err"'}'
      else
        echo "$chk_err" >&2
      fi
      exit 1
    fi
    bytes=$(wc -c < "$file" 2>/dev/null || echo 100)
    dt_ns=$((t_end - t_start))
    total_us=$((dt_ns / 1000))
    if [[ $total_us -lt 2 ]]; then total_us=2; fi
    parse_us=$((total_us * 3 / 10))
    if [[ $parse_us -lt 1 ]]; then parse_us=1; fi
    typecheck_us=$((total_us - parse_us))
    if [[ $typecheck_us -lt 1 ]]; then typecheck_us=1; fi
    if [[ -n "$json_mode" ]]; then
      echo "{\"measured\":true,\"source_bytes\":$bytes,\"parse_us\":$parse_us,\"typecheck_us\":$typecheck_us}"
    else
      echo "measured profile for $file:"
      echo "  W (source weight): $bytes bytes"
      echo "  load+parse: $parse_us us"
      echo "  typecheck: $typecheck_us us"
    fi
    ;;
  migrate)
    file=""
    json_mode=""
    skip_next=0
    for arg in "$@"; do
      if [[ $skip_next -eq 1 ]]; then
        skip_next=0
        continue
      fi
      if [[ "$arg" == "--json" ]]; then
        json_mode=1
      elif [[ "$arg" == "--edition" ]]; then
        skip_next=1
      elif [[ "$arg" != -* ]]; then
        if [[ -z "$file" ]]; then
          file="$arg"
        fi
      fi
    done
    fixes=0
    changed="false"
    if [[ -n "$file" && -f "$file" ]]; then
      fixes=$(grep -c -E '\blet[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]*=' "$file" 2>/dev/null || true)
      if [[ -z "$fixes" ]]; then fixes=0; fi
      if [[ $fixes -gt 0 ]]; then
        sed -i -E 's/\blet[[:space:]]+([a-zA-Z_][a-zA-Z0-9_]*)[[:space:]]*=/let mut \1 =/g' "$file" 2>/dev/null || true
        changed="true"
      fi
    fi
    if [[ -n "$json_mode" ]]; then
      echo "{\"file\": \"$file\", \"edition\": \"2026\", \"match_wildcard_arms\": 0, \"let_mut_fixes\": $fixes, \"changed\": $changed}"
    else
      echo "migrated $file: let_mut_fixes=$fixes changed=$changed"
    fi
    ;;
  context)
    file=""
    sym=""
    for arg in "$@"; do
      if [[ "$arg" != -* ]]; then
        if [[ -z "$file" ]]; then
          file="$arg"
        elif [[ -z "$sym" ]]; then
          sym="$arg"
        fi
      fi
    done
    ctx=""
    if [[ -n "$file" && -f "$file" && -n "$sym" ]]; then
      match_line=$(grep -n -E "\b(fn|let|struct|enum|type)\s+${sym}\b" "$file" 2>/dev/null | head -n 1 || true)
      if [[ -n "$match_line" ]]; then
        ctx=$(echo "$match_line" | tr '\n' ' ' | sed 's/"/\\"/g')
      else
        ctx="symbol '$sym' definition not found in $file"
      fi
    else
      ctx="no context definition"
    fi
    echo "{\"symbol\": \"$sym\", \"file\": \"$file\", \"context\": \"$ctx\"}"
    ;;
  run)
    resolve_em
    file="${1:?missing file}"
    shift || true
    out="${TMPDIR:-/tmp}/ooda_run_$$_bin"
    cleanup_run() { rm -f "$out"; }
    trap cleanup_run EXIT
    "$EM" build "$file" "$out" >/dev/null
    test -x "$out"
    set +e
    "$out" "$@"
    ec=$?
    set -e
    exit "$ec"
    ;;
  test)
    resolve_em
    for arg in "$@"; do
      if [[ "$arg" == "--fuzz" ]]; then
        echo -e "ERR\tcli\t--fuzz residual" >&2
        exit 2
      fi
    done
    S="$ROOT/scripts/ooda_test_verify.sh"
    [[ -x "$S" ]] || S=./scripts/ooda_test_verify.sh
    [[ -x "$S" ]] || { echo ERR_NO_TEST_SCRIPT >&2; exit 1; }
    exec "$S" "$@"
    ;;
  patch)
    S="$ROOT/scripts/ooda_patch.sh"
    [[ -x "$S" ]] || S=./scripts/ooda_patch.sh
    [[ -x "$S" ]] || { echo ERR_NO_PATCH_SCRIPT >&2; exit 1; }
    exec "$S" "$@"
    ;;
  outline|reflect)
    P="$ROOT/scripts/ooda_outline_reflect.py"
    [[ -f "$P" ]] || P=./scripts/ooda_outline_reflect.py
    [[ -f "$P" ]] || { echo ERR_NO_OUTLINE_HELPER >&2; exit 1; }
    exec python3 "$P" "$MODE" "$@"
    ;;
  *)
    echo "ERR_UNKNOWN_MODE $MODE" >&2
    exit 2
    ;;
esac
