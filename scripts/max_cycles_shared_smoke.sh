#!/usr/bin/env bash
# M58 MaxCycles shared per-fn fuel budget (two whiles one counter)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
RT="$ROOT/runtime"
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; exit 1; }

emit_run() {
  local src="$1" tag="$2"
  local c="$TMPDIR/mcs_${tag}.c" bin="$TMPDIR/mcs_${tag}.bin"
  set +e
  "$OODAC" emit-c "$src" >"$c" 2>"$TMPDIR/mcs_${tag}.err"
  local rc=$?
  set -e
  [[ $rc -eq 0 ]] || bad "$tag emit rc=$rc"
  grep -qE $'^ERR\t' "$c" && bad "$tag ERR line" || true
  # one fn-level init, not per-while re-zero brace pattern
  local inits
  inits="$(grep -cE 'long long __oo_mc = 0' "$c" || true)"
  [[ "${inits:-0}" -ge 1 ]] || bad "$tag missing __oo_mc init"
  grep -v '🚀\|Running main' "$c" >"${c}.clean" || true
  mv "${c}.clean" "$c"
  gcc -O0 -I"$RT" "$RT/chs_rt.c" "$c" -o "$bin" -lm
  set +e
  local out r
  out="$(timeout 3 "$bin" 2>&1)"
  r=$?
  set -e
  printf '%s\n' "$out"
  return "$r"
}

out_p="$(emit_run "$ROOT/fixtures/max_cycles_shared_pass.oo" pass)" || true
echo "$out_p" | grep -qE '4' || bad "shared_pass expected 4 got: $out_p"
pass "shared_pass run under budget (4)"

set +e
out_f="$(emit_run "$ROOT/fixtures/max_cycles_shared_fail.oo" fail)"
rf=$?
set -e
[[ $rf -ne 0 ]] || bad "shared_fail should non-zero (per-loop would pass)"
echo "$out_f" | grep -qiE 'max_cycles' || bad "shared_fail missing max_cycles out=$out_f"
pass "shared_fail exceeds combined budget (rc=$rf)"

# M64: mixed while + for share same counter
MIX_FAIL="$ROOT/fixtures/max_cycles_mixed_fail.oo"
MIX_PASS="$ROOT/fixtures/max_cycles_mixed_pass.oo"
if [[ -f "$MIX_PASS" && -f "$MIX_FAIL" ]]; then
  out_mp="$(emit_run "$MIX_PASS" mixp)" || true
  echo "$out_mp" | grep -qE '4' || bad "mixed_pass expected 4 got: $out_mp"
  pass "mixed_pass while+for under budget"
  set +e
  out_mf="$(emit_run "$MIX_FAIL" mixf)"
  rmf=$?
  set -e
  [[ $rmf -ne 0 ]] || bad "mixed_fail should non-zero"
  echo "$out_mf" | grep -qiE 'max_cycles' || bad "mixed_fail missing max_cycles"
  pass "mixed_fail while+for exceed shared budget (rc=$rmf)"
fi


NEST="$ROOT/fixtures/max_cycles_nested_fail.oo"
if [[ -f "$NEST" ]]; then
  set +e
  out_n="$(emit_run "$NEST" nest)"
  rn=$?
  set -e
  [[ $rn -ne 0 ]] || bad "nested_fail should non-zero"
  echo "$out_n" | grep -qiE 'max_cycles' || bad "nested_fail missing max_cycles"
  pass "nested_fail shared budget (rc=$rn)"
fi


HELP="$ROOT/fixtures/max_cycles_helper_fail.oo"
if [[ -f "$HELP" ]]; then
  set +e
  out_h="$(emit_run "$HELP" helpf)"
  rh=$?
  set -e
  [[ $rh -ne 0 ]] || bad "helper_fail should non-zero"
  echo "$out_h" | grep -qiE 'max_cycles' || bad "helper_fail missing max_cycles"
  pass "helper_fail non-main fn fuel (rc=$rh)"
fi

if grep -q 'max_cycles_shared_smoke' "$ROOT/scripts/ci_product.sh"; then
  pass "ci_product wires max_cycles_shared_smoke"
else
  bad "ci_product missing max_cycles_shared_smoke"
fi
echo "max_cycles_shared_smoke: PASSED"
