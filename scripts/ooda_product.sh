#!/usr/bin/env bash
# job: product CLI backend — argv only (no eval of user strings)
# in:  mode + args from pure cli/main.oo (already should be safe; still use "$@")
# out: exit status of oodac / harness
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-}"
shift || true

resolve_em() {
  EM="${OODAC_BIN:-}"
  if [[ -z "$EM" || ! -x "$EM" ]]; then
    if [[ -x "$ROOT/oodac/oodac" ]]; then EM="$ROOT/oodac/oodac"
    elif [[ -x ./oodac/oodac ]]; then EM=./oodac/oodac
    elif [[ -x oodac/oodac ]]; then EM=oodac/oodac
    else echo ERR_NO_OODAC >&2; exit 1
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
    # build <file.oo> [--backend c already applied by caller via extra]
    resolve_em
    file="${1:?missing file}"
    shift || true
    out="${file%.oo}"
    [[ "$file" == *.oo ]] || out="${file}.bin"
    "$EM" build --backend c "$file" "$out" "$@"
    test -x "$out"
    echo "🚀 [openOODA pure oodac] Native executable: $out"
    ;;
  run)
    resolve_em
    file="${1:?missing file}"
    shift || true
    out="${TMPDIR:-/tmp}/ooda_run_$$_bin"
    "$EM" build "$file" "$out" >/dev/null
    test -x "$out"
    set +e
    "$out" "$@"
    ec=$?
    set -e
    rm -f "$out"
    exit "$ec"
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
