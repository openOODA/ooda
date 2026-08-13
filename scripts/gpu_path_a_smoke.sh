#!/usr/bin/env bash
# M165 GPU Path A — noop Ok, cpu:add fallthrough, device residual Err
# emit_ptx/emit_spirv stay free-name residual; no CUDA device product.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
FIX="$ROOT/fixtures/gpu_path_a.oo"

# honesty in runtime: no device shaders; noop/cpu fallthrough present
if grep -q 'gpu residual: no device shaders' runtime/chs_rt_libfloor.c \
  && grep -q 'gpu-noop' runtime/chs_rt_libfloor.c \
  && grep -q 'cpu fallthrough' runtime/chs_rt_libfloor.c; then
  pass "runtime Path A gpu honesty strings present"
else
  bad "runtime Path A gpu honesty missing"
fi
if grep -nE 'cudaLaunch|clEnqueue|vkCmdDispatch' runtime/chs_rt_libfloor.c 2>/dev/null | head -3; then
  bad "must not claim GPU device product"
else
  pass "no GPU device dispatch product"
fi

# free-name residual: emit_ptx / emit_spirv
cat >"$TMPDIR/gpu_ptx.oo" <<'EOF'
pub fn main() { let r = emit_ptx("k"); }
EOF
set +e
"$OODAC" check "$TMPDIR/gpu_ptx.oo" >"$TMPDIR/gpu_ptx.out" 2>"$TMPDIR/gpu_ptx.err"
prc=$?
set -e
if [[ $prc -ne 0 ]] && grep -qE $'ERR\tresidual|Residual product refuse|emit_ptx' \
  "$TMPDIR/gpu_ptx.out" "$TMPDIR/gpu_ptx.err" 2>/dev/null; then
  pass "check residual refuse emit_ptx"
else
  bad "emit_ptx residual refuse missing rc=$prc"
fi

# fixture check + emit + run
set +e
"$OODAC" check "$FIX" >"$TMPDIR/gpa_ck.out" 2>"$TMPDIR/gpa_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/gpa_ck.out"; then
  pass "check gpu_path_a"
else
  bad "check gpu_path_a rc=$ckrc"
  head -8 "$TMPDIR/gpa_ck.out" "$TMPDIR/gpa_ck.err" 2>/dev/null || true
fi

set +e
"$OODAC" emit-c "$FIX" >"$TMPDIR/gpa.c" 2>"$TMPDIR/gpa.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/gpa.c" "$TMPDIR/gpa.err" 2>/dev/null; then
  bad "emit-c gpu_path_a"
  head -12 "$TMPDIR/gpa.err" "$TMPDIR/gpa.c" 2>/dev/null || true
elif ! grep -qE 'oo_gpu_launch\(gpu' "$TMPDIR/gpa.c"; then
  bad "emit missing oo_gpu_launch"
else
  pass "emit-c gpu_path_a lowers"
  gcc "${RT[@]}" "$TMPDIR/gpa.c" -o "$TMPDIR/gpa.bin" 2>"$TMPDIR/gpa.gcc" || {
    bad "gcc gpu_path_a"; head -12 "$TMPDIR/gpa.gcc" || true
  }
  if [[ -x "$TMPDIR/gpa.bin" ]]; then
    out=$("$TMPDIR/gpa.bin" 2>&1) || true
    if echo "$out" | grep -q 'noop-ok' \
      && echo "$out" | grep -q 'cpu-ok' \
      && echo "$out" | grep -q 'device-residual-ok'; then
      pass "runtime noop + cpu fallthrough + device residual"
    else
      bad "runtime out=$out"
    fi
    # forge deny
    sed -E 's/long long gpu = oo_cap_grant_gpu\(\)/long long gpu = 0LL/' \
      "$TMPDIR/gpa.c" >"$TMPDIR/gpa_z.c"
    gcc "${RT[@]}" "$TMPDIR/gpa_z.c" -o "$TMPDIR/gpa_z.bin" 2>/dev/null || true
    if [[ -x "$TMPDIR/gpa_z.bin" ]]; then
      set +e
      zout=$("$TMPDIR/gpa_z.bin" 2>&1) || zrc=$?
      zrc=${zrc:-0}
      set -e
      if [[ $zrc -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
        pass "GpuCap forge zero deny"
      else
        bad "forge not denied out=$zout rc=$zrc"
      fi
    fi
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "gpu_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "gpu_path_a_smoke: PASSED"
exit 0
