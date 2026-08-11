#!/usr/bin/env bash
# M161 ThreadCap/GpuCap path A — dual-run residual honesty (no OS threads / GPU)
# check allow with grant params + emit-c residual Err + forge zero deny
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c")

deny_forge() { # $1=c $2=sed_expr $3=label
  local fc="${1%.c}_forge.c" fb="${1%.c}_forge.bin"
  sed -E "$2" "$1" >"$fc"
  gcc "${RT[@]}" "$fc" -o "$fb" -lm
  set +e; local o rc=0; o=$("$fb" 2>&1) || rc=$?; set -e
  if [[ $rc -ne 0 ]] && echo "$o" | grep -qE $'ERR[\t ]*cap'; then pass "$3"
  else bad "$3 out=$o rc=$rc"; fi
}

THR="$ROOT/fixtures/libfloor_thread_cap.oo"
GPU="$ROOT/fixtures/libfloor_gpu_cap.oo"
[[ -f "$THR" ]] || bad "missing $THR"
[[ -f "$GPU" ]] || bad "missing $GPU"

# --- check allow (sealed + granted) ---
for f in "$THR" "$GPU"; do
  base="$(basename "$f" .oo)"
  set +e
  "$OODAC" check "$f" >"$TMPDIR/lf_${base}.out" 2>"$TMPDIR/lf_${base}.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]] || ! grep -qE '^OK' "$TMPDIR/lf_${base}.out"; then
    bad "check allow $base exit=$rc"
    head -5 "$TMPDIR/lf_${base}.out" "$TMPDIR/lf_${base}.err" 2>/dev/null || true
  else
    pass "check allow $base"
  fi
done

# --- check refuse without cap ---
cat >"$TMPDIR/lf_no_thr.oo" <<'EOF'
pub fn main() { let r = thread_spawn("x"); }
EOF
set +e
"$OODAC" check "$TMPDIR/lf_no_thr.oo" >"$TMPDIR/lf_no_thr.out" 2>"$TMPDIR/lf_no_thr.err"
ntrc=$?
set -e
if [[ $ntrc -eq 0 ]]; then bad "check accepted thread_spawn without ThreadCap"
elif grep -qE 'capability|ERR' "$TMPDIR/lf_no_thr.out" "$TMPDIR/lf_no_thr.err" 2>/dev/null; then
  pass "check deny thread_spawn without ThreadCap"
else
  bad "check deny thread missing ERR exit=$ntrc"
fi

cat >"$TMPDIR/lf_no_gpu.oo" <<'EOF'
pub fn main() { let r = gpu_launch("x"); }
EOF
set +e
"$OODAC" check "$TMPDIR/lf_no_gpu.oo" >"$TMPDIR/lf_no_gpu.out" 2>"$TMPDIR/lf_no_gpu.err"
ngrc=$?
set -e
if [[ $ngrc -eq 0 ]]; then bad "check accepted gpu_launch without GpuCap"
elif grep -qE 'capability|ERR' "$TMPDIR/lf_no_gpu.out" "$TMPDIR/lf_no_gpu.err" 2>/dev/null; then
  pass "check deny gpu_launch without GpuCap"
else
  bad "check deny gpu missing ERR exit=$ngrc"
fi

# --- std wrappers check ---
for m in thread.oo gpu.oo sync.oo; do
  set +e
  "$OODAC" check "$ROOT/std/os/$m" >"$TMPDIR/lf_std_$m.out" 2>"$TMPDIR/lf_std_$m.err"
  src=$?
  set -e
  if [[ $src -ne 0 ]]; then
    bad "check std/os/$m"
    head -5 "$TMPDIR/lf_std_$m.out" "$TMPDIR/lf_std_$m.err" 2>/dev/null || true
  else
    pass "check std/os/$m"
  fi
done

# path A honesty strings + no real OS threads/GPU in residual runtime
if grep -q 'thread_spawn residual: path A seal only' runtime/chs_rt_libfloor.c \
  && grep -q 'mutex residual: path A seal only' runtime/chs_rt_libfloor.c \
  && grep -q 'gpu residual: path A seal only' runtime/chs_rt_libfloor.c; then
  pass "runtime residual err strings present"
else
  bad "runtime residual err strings missing"
fi
if grep -nE 'pthread_create|pthread_mutex|cudaLaunch|clEnqueue|vkCmdDispatch' \
  runtime/chs_rt_libfloor.c 2>/dev/null | head -3; then
  bad "libfloor residual claims real OS/GPU path"
else
  pass "no OS pthread/GPU in libfloor residual"
fi

# --- dual-run: emit-c + residual Err at runtime ---
set +e
"$OODAC" emit-c "$THR" >"$TMPDIR/lf_thr.c" 2>"$TMPDIR/lf_thr.err"
trc=$?
set -e
if [[ $trc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/lf_thr.c" "$TMPDIR/lf_thr.err" 2>/dev/null; then
  bad "emit thread fixture"
  head -8 "$TMPDIR/lf_thr.err" "$TMPDIR/lf_thr.c" 2>/dev/null || true
elif ! grep -qE 'oo_thread_spawn\(thread' "$TMPDIR/lf_thr.c"; then
  bad "emit missing oo_thread_spawn(thread,…)"
elif ! grep -qE 'oo_cap_grant_thread' "$TMPDIR/lf_thr.c"; then
  bad "emit missing oo_cap_grant_thread"
else
  pass "emit thread lowers + grant"
  gcc "${RT[@]}" "$TMPDIR/lf_thr.c" -o "$TMPDIR/lf_thr.bin" -lm 2>"$TMPDIR/lf_thr.gcc" || {
    bad "gcc thread"; head -10 "$TMPDIR/lf_thr.gcc" || true
  }
  if [[ -x "$TMPDIR/lf_thr.bin" ]]; then
    out=$("$TMPDIR/lf_thr.bin" 2>&1) || true
    if echo "$out" | grep -q 'thread-residual-ok' && echo "$out" | grep -q 'mutex-residual-ok'; then
      pass "runtime thread residual Err"
    else
      bad "runtime thread out=$out"
    fi
    deny_forge "$TMPDIR/lf_thr.c" \
      's/long long thread = oo_cap_grant_thread\(\)/long long thread = 0LL/' \
      "runtime Thread forged cap deny (zero)"
  fi
fi

set +e
"$OODAC" emit-c "$GPU" >"$TMPDIR/lf_gpu.c" 2>"$TMPDIR/lf_gpu.err"
grc=$?
set -e
if [[ $grc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/lf_gpu.c" "$TMPDIR/lf_gpu.err" 2>/dev/null; then
  bad "emit gpu fixture"
  head -8 "$TMPDIR/lf_gpu.err" "$TMPDIR/lf_gpu.c" 2>/dev/null || true
elif ! grep -qE 'oo_gpu_launch\(gpu' "$TMPDIR/lf_gpu.c"; then
  bad "emit missing oo_gpu_launch(gpu,…)"
elif ! grep -qE 'oo_cap_grant_gpu' "$TMPDIR/lf_gpu.c"; then
  bad "emit missing oo_cap_grant_gpu"
else
  pass "emit gpu lowers + grant"
  gcc "${RT[@]}" "$TMPDIR/lf_gpu.c" -o "$TMPDIR/lf_gpu.bin" -lm 2>"$TMPDIR/lf_gpu.gcc" || {
    bad "gcc gpu"; head -10 "$TMPDIR/lf_gpu.gcc" || true
  }
  if [[ -x "$TMPDIR/lf_gpu.bin" ]]; then
    gout=$("$TMPDIR/lf_gpu.bin" 2>&1) || true
    if echo "$gout" | grep -q 'gpu-residual-ok'; then
      pass "runtime gpu residual Err"
    else
      bad "runtime gpu out=$gout"
    fi
    deny_forge "$TMPDIR/lf_gpu.c" \
      's/long long gpu = oo_cap_grant_gpu\(\)/long long gpu = 0LL/' \
      "runtime Gpu forged cap deny (zero)"
  fi
fi

# --- emit without cap IDENT fail-closed ---
cat >"$TMPDIR/lf_bad_thr.oo" <<'EOF'
pub fn main(thread: &ThreadCap) { let r = thread_spawn("x"); }
EOF
set +e
"$OODAC" emit-c "$TMPDIR/lf_bad_thr.oo" >"$TMPDIR/lf_bad_thr.c" 2>"$TMPDIR/lf_bad_thr.err"
btrc=$?
set -e
if grep -qE $'^ERR\tc_emit\tthread_spawn requires' "$TMPDIR/lf_bad_thr.c" "$TMPDIR/lf_bad_thr.err" 2>/dev/null; then
  pass "emit thread_spawn without ThreadCap arg fail-closed"
elif [[ $btrc -ne 0 ]]; then pass "emit thread_spawn without cap non-zero exit"
else bad "emit lowered thread_spawn without ThreadCap arg"; fi

cat >"$TMPDIR/lf_bad_gpu.oo" <<'EOF'
pub fn main(gpu: &GpuCap) { let r = gpu_launch("x"); }
EOF
set +e
"$OODAC" emit-c "$TMPDIR/lf_bad_gpu.oo" >"$TMPDIR/lf_bad_gpu.c" 2>"$TMPDIR/lf_bad_gpu.err"
bgrc=$?
set -e
if grep -qE $'^ERR\tc_emit\tgpu_launch requires' "$TMPDIR/lf_bad_gpu.c" "$TMPDIR/lf_bad_gpu.err" 2>/dev/null; then
  pass "emit gpu_launch without GpuCap arg fail-closed"
elif [[ $bgrc -ne 0 ]]; then pass "emit gpu_launch without cap non-zero exit"
else bad "emit lowered gpu_launch without GpuCap arg"; fi

if [[ $fail -ne 0 ]]; then
  echo "libfloor_thread_gpu_smoke: FAILED" >&2
  exit 1
fi
echo "libfloor_thread_gpu_smoke: PASSED"
exit 0
