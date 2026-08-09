#!/usr/bin/env bash
# job: ARC M2 fixtures — emit-c + gcc link chs_rt only + run (exit 0)
# in:  fixtures/arc_smoke/*.oo; tree oodac (seed fallback)
# out: exit 0 if every fixture builds and runs clean
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
TREE="${OODAC_BIN:-$ROOT/oodac/oodac}"
SEED="${SEED_OODAC:-$ROOT/bootstrap/seed/oodac}"
RT="$ROOT/runtime/chs_rt.c"
INC="$ROOT/runtime"
DIR="$ROOT/fixtures/arc_smoke"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

emit_c() {
  local src="$1" out="$2" err="$3" host="$4"
  set +e
  timeout 30 "$host" emit-c "$src" >"$out" 2>"$err"
  local rc=$?
  set -e
  if [[ $rc -ne 0 || ! -s "$out" ]] || grep -qE $'^ERR\t' "$out" 2>/dev/null; then
    return 1
  fi
  # Drop host noise lines if any
  grep -v '🚀\|Running main' "$out" >"${out}.clean" 2>/dev/null || true
  if [[ -s "${out}.clean" ]]; then mv "${out}.clean" "$out"; else rm -f "${out}.clean"; fi
  return 0
}

n=0
for src in "$DIR"/*.oo; do
  [[ -f "$src" ]] || continue
  n=$((n + 1))
  base="$(basename "$src" .oo)"
  c_out="$TMPDIR/arc_${base}.c"
  err_out="$TMPDIR/arc_${base}.err"
  bin_out="$TMPDIR/arc_${base}.bin"
  used="tree"
  if [[ -x "$TREE" ]] && emit_c "$src" "$c_out" "$err_out" "$TREE"; then
    used="tree"
  elif [[ -x "$SEED" ]] && emit_c "$src" "$c_out" "$err_out" "$SEED"; then
    used="seed"
  else
    bad "emit-c $base"
    head -12 "$err_out" 2>/dev/null || true
    continue
  fi
  set +e
  gcc -O0 -I"$INC" "$RT" "$c_out" -o "$bin_out" -lm >"$TMPDIR/arc_${base}.gcc" 2>&1
  grc=$?
  set -e
  if [[ $grc -ne 0 || ! -x "$bin_out" ]]; then
    bad "gcc $base"
    head -12 "$TMPDIR/arc_${base}.gcc" 2>/dev/null || true
    continue
  fi
  set +e
  timeout 5 "$bin_out" >"$TMPDIR/arc_${base}.out" 2>"$TMPDIR/arc_${base}.runerr"
  rrc=$?
  set -e
  if [[ $rrc -ne 0 ]]; then
    bad "run $base exit=$rrc"
    cat "$TMPDIR/arc_${base}.runerr" 2>/dev/null || true
    continue
  fi
  # early_return_string: ab then b (println String path)
  if [[ "$base" == "early_return_string" ]]; then
    got="$(cat "$TMPDIR/arc_${base}.out")"
    exp=$'ab\nb'
    if [[ "$got" != "$exp" ]]; then
      bad "early_return_string output want ab/b got: $(printf '%q' "$got")"
      continue
    fi
  fi
  if [[ "$base" == "string_concat_reassign" ]]; then
    got="$(cat "$TMPDIR/arc_${base}.out")"
    if [[ "$got" != "hello world!" ]]; then
      bad "string_concat_reassign output got: $(printf '%q' "$got")"
      continue
    fi
  fi
  pass "$base ($used)"
done

if [[ "$n" -eq 0 ]]; then
  echo "ERR no fixtures under $DIR" >&2
  exit 1
fi
if [[ $fail -ne 0 ]]; then
  echo "arc_smoke: FAILED" >&2
  exit 1
fi
echo "arc_smoke: PASSED ($n fixtures)"
exit 0
