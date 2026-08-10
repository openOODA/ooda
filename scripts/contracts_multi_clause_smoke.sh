#!/usr/bin/env bash
# job: M51 multi-clause simple contracts (AND of simple requires/ensures)
# stage: test
# residual: complex (&& / expr / SMT) still fail-closed via contracts_native_smoke
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

MC_PASS="$ROOT/fixtures/multi_clause_pass.oo"
MC_REQ_FAIL="$ROOT/fixtures/multi_clause_req_fail.oo"
MC_ENS_FAIL="$ROOT/fixtures/multi_clause_ens_fail.oo"
RT="$ROOT/runtime"

emit_ok() {
  local src="$1" base c_out
  base="$(basename "$src" .oo)"
  c_out="$TMPDIR/mc_${base}.c"
  set +e
  "$OODAC" emit-c "$src" >"$c_out" 2>"$TMPDIR/mc_${base}.err"
  local rc=$?
  set -e
  if [[ "$rc" != "0" ]] || grep -E $'^ERR\t' "$c_out" >/dev/null 2>&1; then
    echo "FAIL multi-clause emit: $src" >&2
    cat "$TMPDIR/mc_${base}.err" >&2 || true
    grep -E $'^ERR\t' "$c_out" >&2 || true
    exit 1
  fi
  grep -v '🚀\|Running main' "$c_out" >"${c_out}.clean" || true
  mv "${c_out}.clean" "$c_out"
  printf '%s' "$c_out"
}

# pass: ≥2 requires + ≥2 ensures lowers; run → 3
c_mc="$(emit_ok "$MC_PASS")"
req_n="$(grep -cE 'if \(!\(' "$c_mc" || true)"
[[ "${req_n:-0}" -ge 2 ]] || { echo "FAIL multi_clause_pass requires lowers=$req_n" >&2; exit 1; }
grep -q 'OO_ENS_CHECK' "$c_mc" || {
  echo "FAIL multi_clause_pass missing multi-ensures lower" >&2; exit 1; }
bin_mc="$TMPDIR/multi_clause_pass.bin"
gcc -O0 -I"$RT" "$RT/chs_rt.c" "$c_mc" -o "$bin_mc" -lm
out_mc="$(timeout 3 "$bin_mc")"
echo "$out_mc" | grep -qE '3' || {
  echo "FAIL multi_clause_pass run expected 3 got: $out_mc" >&2; exit 1; }
echo "OK multi-clause pass multi_clause_pass"

# second requires fails at runtime
c_mrf="$(emit_ok "$MC_REQ_FAIL")"
bin_mrf="$TMPDIR/multi_clause_req_fail.bin"
gcc -O0 -I"$RT" "$RT/chs_rt.c" "$c_mrf" -o "$bin_mrf" -lm
set +e; out_mrf="$(timeout 3 "$bin_mrf" 2>&1)"; mrfrc=$?; set -e
[[ $mrfrc -ne 0 ]] || { echo "FAIL multi_clause_req_fail should non-zero" >&2; exit 1; }
echo "$out_mrf" | grep -qiE 'contract|requires' || {
  echo "FAIL multi_clause_req_fail needle out=$out_mrf" >&2; exit 1; }
echo "OK multi-clause req fail multi_clause_req_fail (rc=$mrfrc)"

# second ensures fails at runtime
c_mef="$(emit_ok "$MC_ENS_FAIL")"
grep -q 'OO_ENS_CHECK' "$c_mef" || { echo "FAIL multi_clause_ens_fail ensures setup" >&2; exit 1; }
bin_mef="$TMPDIR/multi_clause_ens_fail.bin"
gcc -O0 -I"$RT" "$RT/chs_rt.c" "$c_mef" -o "$bin_mef" -lm
set +e; out_mef="$(timeout 3 "$bin_mef" 2>&1)"; mefrc=$?; set -e
[[ $mefrc -ne 0 ]] || { echo "FAIL multi_clause_ens_fail should non-zero" >&2; exit 1; }
echo "$out_mef" | grep -qiE 'contract|ensures' || {
  echo "FAIL multi_clause_ens_fail needle out=$out_mef" >&2; exit 1; }
echo "OK multi-clause ens fail multi_clause_ens_fail (rc=$mefrc)"


# M67 three-clause depth
THREE="$ROOT/fixtures/multi_clause_three.oo"
THREE_F="$ROOT/fixtures/multi_clause_three_req_fail.oo"
if [[ -f "$THREE" ]]; then
  c3="$(emit_ok "$THREE")"
  r3="$(grep -cE 'if \(!\(' "$c3" || true)"
  [[ "${r3:-0}" -ge 3 ]] || { echo "FAIL three requires lowers=$r3" >&2; exit 1; }
  grep -q 'OO_ENS_CHECK' "$c3" || { echo "FAIL three ensures setup" >&2; exit 1; }
  bin3="$TMPDIR/multi_clause_three.bin"
  gcc -O0 -I"$RT" "$RT/chs_rt.c" "$c3" -o "$bin3" -lm
  out3="$(timeout 3 "$bin3")"
  echo "$out3" | grep -qE '6' || { echo "FAIL three run expected 6 got $out3" >&2; exit 1; }
  echo "OK multi-clause three-clause pass"
fi
if [[ -f "$THREE_F" ]]; then
  c3f="$(emit_ok "$THREE_F")"
  bin3f="$TMPDIR/multi_clause_three_req_fail.bin"
  gcc -O0 -I"$RT" "$RT/chs_rt.c" "$c3f" -o "$bin3f" -lm
  set +e; out3f="$(timeout 3 "$bin3f" 2>&1)"; r3f=$?; set -e
  [[ $r3f -ne 0 ]] || { echo "FAIL three_req_fail should non-zero" >&2; exit 1; }
  echo "$out3f" | grep -qiE 'contract|requires' || { echo "FAIL three_req_fail needle" >&2; exit 1; }
  echo "OK multi-clause three-clause req fail (rc=$r3f)"
fi


# M72 ensures cap 8 overflow fail-closed is no longer applicable with OO_ENS_CHECK

echo "contracts_multi_clause_smoke: pass+fail OK"
