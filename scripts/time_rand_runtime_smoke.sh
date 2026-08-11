#!/usr/bin/env bash
# Runtime sleep_ms + seed (grant inject names: time / rand; link chs_rt.c only)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; exit 1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c")

run_one() {
  local name="$1" src="$2" needle="$3" out_needle="$4"
  local c="$TMPDIR/${name}.c" bin="$TMPDIR/${name}.bin"
  cat >"$TMPDIR/${name}.oo" <<<"$src"
  # use heredoc properly via file write from caller
}
# sleep
cat >"$TMPDIR/tr_sleep.oo" <<'OO'
pub fn main(time: &TimeCap) {
    sleep_ms(time, 1);
    println("sleep-ok");
}
OO
set +e; "$OODAC" emit-c "$TMPDIR/tr_sleep.oo" >"$TMPDIR/tr_sleep.c" 2>"$TMPDIR/tr_sleep.err"; e=$?; set -e
[[ $e -eq 0 ]] || bad "emit sleep"
grep -q oo_sleep_ms "$TMPDIR/tr_sleep.c" || bad "missing oo_sleep_ms"
gcc "${RT[@]}" "$TMPDIR/tr_sleep.c" -o "$TMPDIR/tr_sleep.bin" -lm 2>"$TMPDIR/tr_sleep.gcc" || { cat "$TMPDIR/tr_sleep.gcc"; bad gcc-sleep; }
out=$("$TMPDIR/tr_sleep.bin" 2>&1) || true
echo "$out" | grep -q sleep-ok && pass "runtime sleep_ms under TimeCap" || bad "sleep out=$out"

# seed
cat >"$TMPDIR/tr_seed.oo" <<'OO'
pub fn main(rand: &RandCap) {
    seed(rand, 42);
    println("seed-ok");
}
OO
set +e; "$OODAC" emit-c "$TMPDIR/tr_seed.oo" >"$TMPDIR/tr_seed.c" 2>"$TMPDIR/tr_seed.err"; e=$?; set -e
[[ $e -eq 0 ]] || bad "emit seed"
grep -q oo_seed "$TMPDIR/tr_seed.c" || bad "missing oo_seed"
gcc "${RT[@]}" "$TMPDIR/tr_seed.c" -o "$TMPDIR/tr_seed.bin" -lm 2>"$TMPDIR/tr_seed.gcc" || { cat "$TMPDIR/tr_seed.gcc"; bad gcc-seed; }
out=$("$TMPDIR/tr_seed.bin" 2>&1) || true
echo "$out" | grep -q seed-ok && pass "runtime seed under RandCap" || bad "seed out=$out"

echo "OK time_rand_runtime_smoke"
