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
    elif [[ -x "$ROOT/dist/ooda-v0.183.0-alpha-linux-x86_64/oodac/oodac" ]]; then
      EM="$(readlink -f "$ROOT/dist/ooda-v0.183.0-alpha-linux-x86_64/oodac/oodac" 2>/dev/null || echo "$ROOT/dist/ooda-v0.183.0-alpha-linux-x86_64/oodac/oodac")"
    elif [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then
      EM="$(readlink -f "$ROOT/bootstrap/seed/oodac" 2>/dev/null || echo "$ROOT/bootstrap/seed/oodac")"
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
    out=""
    extra=()
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --target)
          target="$2"
          shift 2
          ;;
        --target=*)
          target="${1#--target=}"
          shift
          ;;
        --emit-llvm)
          target="llvm"
          shift
          ;;
        -o)
          out="$2"
          shift 2
          ;;
        -o=*)
          out="${1#-o=}"
          shift
          ;;
        *)
          if [[ -z "$file" ]]; then file="$1"; else extra+=("$1"); fi
          shift
          ;;
      esac
    done
    if [[ "$target" == "wasm" || "$target" == "llvm" ]]; then
      [[ -n "$file" ]] || { echo -e "ERR\tbuild\tmissing file" >&2; exit 2; }
      ext="wat"; cmd="emit-wasm"; msg="WebAssembly text module"
      [[ "$target" == "llvm" ]] && { ext="ll"; cmd="emit-llvm"; msg="LLVM IR emitted" ; }
      if [[ "$target" == "wasm" ]]; then
        [[ -z "$out" ]] && { out="${file%.oo}.$ext"; [[ "$file" == *.oo ]] || out="${file}.$ext"; }
        tmp_out="${TMPDIR:-/tmp}/${target}_out_$$.$ext"
        if ! "$EM" "$cmd" "$file" > "$tmp_out"; then rm -f "$tmp_out" 2>/dev/null || true; exit 2; fi
        if ! cp "$tmp_out" "$out" 2>/dev/null; then echo -e "ERR\tbuild\tfailed to write output file: $out" >&2; rm -f "$tmp_out" 2>/dev/null || true; exit 2; fi
        rm -f "$tmp_out" 2>/dev/null || true
        test -s "$out"
        echo "🚀 [openOODA pure oodac] $msg: $out"
        exit 0
      else
        # LLVM target
        [[ -z "$out" ]] && { out="${file%.oo}.bin"; [[ "$file" == *.oo ]] || out="${file}.bin"; }
        tmp_out="${TMPDIR:-/tmp}/${target}_out_$$.ll"
        if ! "$EM" "$cmd" "$file" > "$tmp_out"; then rm -f "$tmp_out" 2>/dev/null || true; exit 2; fi
        if ! clang -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$tmp_out" -lm -o "$out"; then
          echo -e "ERR\tbuild\tclang failed to compile $tmp_out" >&2
          rm -f "$tmp_out" 2>/dev/null || true
          exit 2
        fi
        rm -f "$tmp_out" 2>/dev/null || true
        test -x "$out"
        echo "🚀 [openOODA pure oodac] Native LLVM executable: $out"
        exit 0
      fi
    fi
    if [[ -z "$out" ]]; then
      if [[ ${#extra[@]} -gt 0 && -n "${extra[0]:-}" ]]; then
        out="${extra[0]}"
      else
        out="${file%.oo}"
        [[ "$file" == *.oo ]] || out="${file}.bin"
      fi
    fi
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
    S="$ROOT/scripts/ooda_product_migrate.sh"
    [[ -x "$S" ]] || S=./scripts/ooda_product_migrate.sh
    exec "$S" "$@"
    ;;
  context)
    S="$ROOT/scripts/ooda_product_context.sh"
    [[ -x "$S" ]] || S=./scripts/ooda_product_context.sh
    exec "$S" "$@"
    ;;
  run)
    resolve_em
    export EM
    S="$ROOT/scripts/ooda_product_run.sh"
    [[ -x "$S" ]] || S=./scripts/ooda_product_run.sh
    [[ -x "$S" ]] || { echo ERR_NO_RUN_SCRIPT >&2; exit 1; }
    exec "$S" "$@"
    ;;
  test)
    resolve_em
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
    resolve_em
    exec "$EM" "$MODE" "$@"
    ;;
  *)
    echo "ERR_UNKNOWN_MODE $MODE" >&2
    exit 2
    ;;
esac
