#!/usr/bin/env bash
# job: pass fixtures for oodac emit-c (control-flow slice)
# stage: test
# in:  bootstrap/corpus/emit-c/pass/*.oo
# out: exit 0 if each emits C, gcc-links with chs_rt, runs
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA:-$ROOT/target/release/ooda}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODA" ]]; then
  (cd "$ROOT" && cargo build --release)
fi

PASS_DIR="$ROOT/bootstrap/corpus/emit-c/pass"
n=0
for src in "$PASS_DIR"/*.oo; do
  [[ -f "$src" ]] || continue
  n=$((n + 1))
  base="$(basename "$src" .oo)"
  c_out="$TMPDIR/emit_${base}.c"
  bin_out="$TMPDIR/emit_${base}.bin"
  # Host interpreter expands oodac multi-file imports for emit-c CLI.
  "$OODA" run "$ROOT/oodac/main.oo" -- emit-c "$src" >"$c_out" 2>"$TMPDIR/emit_${base}.err" || {
    echo "FAIL emit-c exit: $src" >&2
    cat "$TMPDIR/emit_${base}.err" >&2
    exit 1
  }
  if grep -E $'^ERR\t' "$c_out" >/dev/null 2>&1; then
    echo "FAIL emit-c ERR line: $src" >&2
    grep -E $'^ERR\t' "$c_out" >&2 || true
    exit 1
  fi
  # Drop host runner banner lines if any leaked
  grep -v '🚀\|Running main' "$c_out" >"${c_out}.clean" || true
  mv "${c_out}.clean" "$c_out"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_out" -o "$bin_out" -lm
  "$bin_out" >/dev/null
  echo "OK emit-c $base"
done

if [[ "$n" -eq 0 ]]; then
  echo "no pass fixtures under $PASS_DIR" >&2
  exit 1
fi
echo "c_emit_smoke: $n pass fixture(s) OK"
