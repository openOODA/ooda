#!/usr/bin/env bash
# job: pass+fail fixtures for oodac emit-c
# stage: test
# in:  bootstrap/corpus/emit-c/{pass,fail}/*.oo
# out: exit 0 if pass emit+gcc+run and fail produce ERR\tc_emit or non-zero
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA:-$ROOT/target/release/ooda}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODA" ]]; then
  (cd "$ROOT" && cargo build --release)
fi

run_emit() {
  # writes C (or ERR) to $2; prints ooda exit code on stdout
  local src="$1" out="$2" err="$3"
  set +e
  "$OODA" run "$ROOT/oodac/main.oo" -- emit-c "$src" >"$out" 2>"$err"
  local rc=$?
  set +e
  printf '%s' "$rc"
}

PASS_DIR="$ROOT/bootstrap/corpus/emit-c/pass"
n_pass=0
for src in "$PASS_DIR"/*.oo; do
  [[ -f "$src" ]] || continue
  [[ "$src" == *.concat.oo ]] && continue
  n_pass=$((n_pass + 1))
  base="$(basename "$src" .oo)"
  c_out="$TMPDIR/emit_${base}.c"
  bin_out="$TMPDIR/emit_${base}.bin"
  rc="$(run_emit "$src" "$c_out" "$TMPDIR/emit_${base}.err")"
  if [[ "$rc" != "0" ]]; then
    echo "FAIL emit-c exit $rc: $src" >&2
    cat "$TMPDIR/emit_${base}.err" >&2
    exit 1
  fi
  if grep -E $'^ERR\t' "$c_out" >/dev/null 2>&1; then
    echo "FAIL emit-c ERR line: $src" >&2
    grep -E $'^ERR\t' "$c_out" >&2 || true
    exit 1
  fi
  grep -v '🚀\|Running main' "$c_out" >"${c_out}.clean" || true
  mv "${c_out}.clean" "$c_out"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_out" -o "$bin_out" -lm
  # Bound runaway loops from incomplete while-emit (fail closed if hangs).
  timeout 3 "$bin_out" >/dev/null || {
    echo "FAIL emit-c run/timeout: $src" >&2
    exit 1
  }
  echo "OK emit-c pass $base"
done

if [[ "$n_pass" -eq 0 ]]; then
  echo "no pass fixtures under $PASS_DIR" >&2
  exit 1
fi

FAIL_DIR="$ROOT/bootstrap/corpus/emit-c/fail"
n_fail=0
for src in "$FAIL_DIR"/*.oo; do
  [[ -f "$src" ]] || continue
  [[ "$src" == *.concat.oo ]] && continue
  n_fail=$((n_fail + 1))
  base="$(basename "$src" .oo)"
  c_out="$TMPDIR/emit_fail_${base}.c"
  err_out="$TMPDIR/emit_fail_${base}.err"
  rc="$(run_emit "$src" "$c_out" "$err_out")"
  if grep -E $'^ERR\tc_emit' "$c_out" "$err_out" >/dev/null 2>&1; then
    echo "OK emit-c fail $base (ERR line)"
    continue
  fi
  if [[ "$rc" != "0" ]]; then
    echo "OK emit-c fail $base (exit $rc)"
    continue
  fi
  echo "FAIL emit-c should reject: $src (exit 0, no ERR)" >&2
  head -20 "$c_out" >&2
  exit 1
done

if [[ "$n_fail" -eq 0 ]]; then
  echo "no fail fixtures under $FAIL_DIR" >&2
  exit 1
fi

echo "c_emit_smoke: $n_pass pass + $n_fail fail fixture(s) OK"
