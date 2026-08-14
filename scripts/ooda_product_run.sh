#!/usr/bin/env bash
# job: product run --engine bc|native (M11)
# in:  OODAC_BIN/EM from caller; args after `run`
# out: exec program; fail-closed on bad engine/build
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EM="${EM:-${OODAC_BIN:-$ROOT/oodac/oodac}}"
if [[ ! -x "$EM" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then EM="$ROOT/oodac/oodac"
  else echo "ERR_NO_OODAC" >&2; exit 1; fi
fi

engine=native
file=""
echo "ooda_product_run.sh args: $@" >&2
prog=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --engine)
      shift
      engine="${1:-}"
      if [[ -z "$engine" ]]; then
        echo -e "ERR\trun\t--engine needs value" >&2
        exit 2
      fi
      shift
      ;;
    --engine=*)
      engine="${1#--engine=}"
      shift
      ;;
    --)
      shift
      prog+=("$@")
      break
      ;;
    -*)
      echo -e "ERR\trun\tunsupported flag: $1" >&2
      exit 2
      ;;
    *)
      if [[ -z "$file" ]]; then
        file="$1"
      else
        prog+=("$1")
      fi
      shift
      ;;
  esac
done
if [[ -z "$file" ]]; then
  echo -e "ERR\trun\tmissing file" >&2
  exit 2
fi
case "$engine" in
  bc)
    exec "$EM" run "$file"
    ;;
  native)
    out="${TMPDIR:-/tmp}/ooda_run_$$"
    trap 'rm -f "$out"' EXIT
    set +e
    "$EM" build --backend c "$file" "$out" >/dev/null 2>"${out}.build.err"
    brc=$?
    set -e
    if [[ $brc -ne 0 || ! -x "$out" ]]; then
      echo -e "ERR\trun\tnative build failed" >&2
      head -20 "${out}.build.err" 2>/dev/null || true
      exit 1
    fi
    set +e
    if [[ ${#prog[@]} -gt 0 ]]; then
      echo "ENV BEFORE OUT:" >&2
      env | grep OODA_ >&2
      "$out" "${prog[@]}"
      rrc=$?
    else
      echo "ENV BEFORE OUT:" >&2
      env | grep OODA_ >&2
      "$out"
      rrc=$?
    fi
    set -e
    exit "$rrc"
    ;;
  *)
    echo -e "ERR\trun\tinvalid engine: $engine (want bc|native)" >&2
    exit 2
    ;;
esac
