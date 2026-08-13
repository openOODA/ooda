#!/usr/bin/env bash
# M161 libfloor path A — ThreadCap mutex/thread_spawn + GpuCap gpu_launch residual seals
# Dual path: granted → Result residual Err; forge (zero/magic) → ERR cap deny
# Honesty: no OS pthreads/spinlocks/GPU shaders
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)

# check: bare mutex_lock without ThreadCap refused
cat >"$TMPDIR/lf_bare_mutex.oo" <<'EOF'
pub fn main() { let r = mutex_lock(1, 1); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/lf_bare_mutex.oo" >"$TMPDIR/lf_bare.out" 2>"$TMPDIR/lf_bare.err"
brc=$?
set -e
if [[ $brc -ne 0 ]] && grep -qiE 'capability|ThreadCap|ERR' "$TMPDIR/lf_bare.out" "$TMPDIR/lf_bare.err" 2>/dev/null; then
  pass "check refuse bare mutex_lock"
else
  bad "bare mutex_lock accepted rc=$brc"
fi

# check: granted ThreadCap + mutex_lock / thread_spawn pass
set +e
"$OODAC_BIN" check "$ROOT/fixtures/libfloor_mutex.oo" >"$TMPDIR/lf_ck_m.out" 2>"$TMPDIR/lf_ck_m.err"
mrc=$?
"$OODAC_BIN" check "$ROOT/fixtures/libfloor_thread_spawn.oo" >"$TMPDIR/lf_ck_t.out" 2>"$TMPDIR/lf_ck_t.err"
trc=$?
set -e
[[ $mrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/lf_ck_m.out" && pass "check mutex with ThreadCap" || bad "check mutex"
[[ $trc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/lf_ck_t.out" && pass "check thread_spawn with ThreadCap" || bad "check thread_spawn"

# std wrapper library check (single-file)
set +e
"$OODAC_BIN" check "$ROOT/std/os/sync.oo" >"$TMPDIR/lf_ck_s.out" 2>"$TMPDIR/lf_ck_s.err"
src=$?
set -e
[[ $src -eq 0 ]] && grep -qE '^OK' "$TMPDIR/lf_ck_s.out" && pass "check std/os/sync.oo" || {
  bad "check std/os/sync.oo"
  head -10 "$TMPDIR/lf_ck_s.out" "$TMPDIR/lf_ck_s.err" || true
}

emit_run() {
  # $1=fixture $2=binbase $3=needle $4=label $5=grant_sed_ident
  local fix="$1" base="$2" needle="$3" label="$4" gident="$5"
  set +e
  "$OODAC_BIN" emit-c "$fix" >"$TMPDIR/${base}.c" 2>"$TMPDIR/${base}.err"
  local erc=$?
  set -e
  if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/${base}.c" "$TMPDIR/${base}.err" 2>/dev/null; then
    bad "emit-c $label"
    head -15 "$TMPDIR/${base}.err" "$TMPDIR/${base}.c" || true
    return
  fi
  pass "emit-c $label"
  if ! grep -q "oo_cap_grant_${gident}" "$TMPDIR/${base}.c"; then
    bad "emit missing grant_$gident"
  else
    pass "emit grant_$gident"
  fi
  gcc "${RT[@]}" "$TMPDIR/${base}.c" -o "$TMPDIR/${base}.bin" 2>"$TMPDIR/${base}_gcc.err" || {
    bad "gcc $label"
    head -20 "$TMPDIR/${base}_gcc.err" || true
    return
  }
  local out
  out=$("$TMPDIR/${base}.bin" 2>&1) || true
  if echo "$out" | grep -q "$needle"; then
    pass "runtime residual $label"
  else
    bad "runtime $label out=$out"
  fi
  # forge deny: zero token
  sed -E "s/long long ${gident} = oo_cap_grant_${gident}\\(\\)/long long ${gident} = 0LL/" \
    "$TMPDIR/${base}.c" >"$TMPDIR/${base}_zero.c"
  gcc "${RT[@]}" "$TMPDIR/${base}_zero.c" -o "$TMPDIR/${base}_zero.bin" -lm -ldl -lpthread
  set +e
  local zout zrc=0
  zout=$("$TMPDIR/${base}_zero.bin" 2>&1) || zrc=$?
  set -e
  if [[ $zrc -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
    pass "zero forge deny $label"
  else
    bad "zero forge $label out=$zout rc=$zrc"
  fi
  # classic magic forge (thread 0x4F4F5448 "OOTH", gpu 0x4F4F4750 "OOGP")
  local magic="0x4F4F5448LL"
  [[ "$gident" == "gpu" ]] && magic="0x4F4F4750LL"
  sed -E "s/long long ${gident} = oo_cap_grant_${gident}\\(\\)/long long ${gident} = ${magic}/" \
    "$TMPDIR/${base}.c" >"$TMPDIR/${base}_mag.c"
  gcc "${RT[@]}" "$TMPDIR/${base}_mag.c" -o "$TMPDIR/${base}_mag.bin" -lm -ldl -lpthread
  set +e
  local mout mrc2=0
  mout=$("$TMPDIR/${base}_mag.bin" 2>&1) || mrc2=$?
  set -e
  if [[ $mrc2 -ne 0 ]] && echo "$mout" | grep -qE $'ERR[\t ]*cap'; then
    pass "magic forge deny $label"
  else
    bad "magic forge $label out=$mout rc=$mrc2"
  fi
}

emit_run "$ROOT/fixtures/libfloor_mutex.oo" "lf_mutex" "mutex-lock-ok" "mutex" "thread"
emit_run "$ROOT/fixtures/libfloor_thread_spawn.oo" "lf_tspawn" "thread-spawn-ok" "thread_spawn" "thread"
emit_run "$ROOT/fixtures/libfloor_gpu_launch.oo" "lf_gpu" "gpu-residual-ok" "gpu_launch" "gpu"

# M162/M163: real pthread mutex + joinable spawn; GPU residual remains
if grep -q 'pthread_mutex_lock' runtime/chs_rt_libfloor.c \
  && grep -q 'pthread_create' runtime/chs_rt_thread.c \
  && grep -q 'pthread_join' runtime/chs_rt_thread.c \
  && grep -q 'gpu residual: no device shaders' runtime/chs_rt_libfloor.c; then
  pass "runtime pthread path A + GPU residual present"
else
  bad "runtime thread/gpu path A missing"
fi
if grep -nE 'cudaLaunch|clEnqueue|vkCmdDispatch' runtime/chs_rt_libfloor.c runtime/chs_rt_thread.c 2>/dev/null | head -3; then
  bad "libfloor must not claim GPU shader product"
else
  pass "no GPU shader product in libfloor"
fi

if [[ $fail -ne 0 ]]; then
  echo "libfloor_mutex_thread_smoke: FAILED" >&2
  exit 1
fi
echo "libfloor_mutex_thread_smoke: PASSED"
exit 0
