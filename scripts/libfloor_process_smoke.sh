#!/usr/bin/env bash
# job: libfloor path A sys_spawn/wait/kill residual + SysCap dual + process_exec real
# in:  oodac, fixtures/libfloor_sys_spawn.oo, bootstrap corpus, std/os/process.oo
# out: exit 0 if seal/check/runtime honesty holds
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
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c")

FAIL="$ROOT/bootstrap/corpus/check/fail/no_cap_sys_spawn.oo"
PASS="$ROOT/bootstrap/corpus/check/pass/ok_sys_spawn.oo"
FIX="$ROOT/fixtures/libfloor_sys_spawn.oo"
STD="$ROOT/std/os/process.oo"

[[ -f "$FAIL" ]] || bad "missing $FAIL"
[[ -f "$PASS" ]] || bad "missing $PASS"
[[ -f "$FIX" ]] || bad "missing $FIX"
[[ -f "$STD" ]] || bad "missing $STD"

# Dual: bare sys_spawn without cap fails check
set +e
"$OODAC" check "$FAIL" >"$TMPDIR/lp_fail.out" 2>"$TMPDIR/lp_fail.err"
frc=$?
set -e
if [[ $frc -eq 0 ]]; then
  bad "check accepted no_cap_sys_spawn"
elif ! grep -qE 'capability|ERR' "$TMPDIR/lp_fail.out" "$TMPDIR/lp_fail.err" 2>/dev/null; then
  bad "check fail no_cap_sys_spawn missing ERR (exit=$frc)"
else
  pass "check deny no_cap_sys_spawn"
fi

# Dual: with &SysCap, check allows (runtime residual separate)
set +e
"$OODAC" check "$PASS" >"$TMPDIR/lp_pass.out" 2>"$TMPDIR/lp_pass.err"
prc=$?
set -e
if [[ $prc -ne 0 ]] || ! grep -qE '^OK' "$TMPDIR/lp_pass.out"; then
  bad "check rejected ok_sys_spawn exit=$prc"
  head -8 "$TMPDIR/lp_pass.out" "$TMPDIR/lp_pass.err" 2>/dev/null || true
else
  pass "check allow ok_sys_spawn"
fi

# std thin wrappers check
set +e
"$OODAC" check "$STD" >"$TMPDIR/lp_std.out" 2>"$TMPDIR/lp_std.err"
src=$?
set -e
if [[ $src -ne 0 ]]; then
  bad "check std/os/process.oo"
  head -12 "$TMPDIR/lp_std.out" "$TMPDIR/lp_std.err" 2>/dev/null || true
else
  pass "check std/os/process.oo"
fi

# emit residual fixture → oo_sys_spawn + runtime Err residual
set +e
"$OODAC" emit-c "$FIX" >"$TMPDIR/lp_spawn.c" 2>"$TMPDIR/lp_spawn.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/lp_spawn.c" "$TMPDIR/lp_spawn.err" 2>/dev/null; then
  bad "emit-c libfloor_sys_spawn"
  head -20 "$TMPDIR/lp_spawn.err" "$TMPDIR/lp_spawn.c" 2>/dev/null || true
else
  pass "emit-c libfloor_sys_spawn"
fi
if grep -qE 'oo_sys_spawn\(' "$TMPDIR/lp_spawn.c" \
  && grep -qE 'oo_sys_wait\(' "$TMPDIR/lp_spawn.c" \
  && grep -qE 'oo_sys_kill\(' "$TMPDIR/lp_spawn.c" \
  && grep -qE 'oo_cap_grant_sys' "$TMPDIR/lp_spawn.c"; then
  pass "emit lowers oo_sys_spawn/wait/kill + grant_sys"
else
  bad "missing oo_sys_* / grant_sys lowers"
  grep -nE 'sys_|spawn|wait|kill|grant' "$TMPDIR/lp_spawn.c" | head -20 || true
fi

# emit without cap IDENT first arg must fail-closed
cat >"$TMPDIR/lp_bad_spawn.oo" <<'EOF'
pub fn main(sys: &SysCap) { let r = sys_spawn("true"); }
EOF
set +e
"$OODAC" emit-c "$TMPDIR/lp_bad_spawn.oo" >"$TMPDIR/lp_bad_spawn.c" 2>"$TMPDIR/lp_bad_spawn.err"
bsc=$?
set -e
if grep -qE $'^ERR\tc_emit\tsys_spawn requires' "$TMPDIR/lp_bad_spawn.c" "$TMPDIR/lp_bad_spawn.err" 2>/dev/null; then
  pass "emit sys_spawn without SysCap arg fail-closed"
elif [[ $bsc -ne 0 ]]; then
  pass "emit sys_spawn without cap non-zero exit"
else
  bad "emit lowered sys_spawn without SysCap arg"
fi

if [[ -f "$TMPDIR/lp_spawn.c" ]] && ! grep -qE $'^ERR\t' "$TMPDIR/lp_spawn.c" 2>/dev/null; then
  gcc "${RT[@]}" "$TMPDIR/lp_spawn.c" -o "$TMPDIR/lp_spawn.bin" -lm -ldl -lpthread 2>"$TMPDIR/lp_gcc.err" || {
    bad "gcc link libfloor_sys_spawn"
    head -20 "$TMPDIR/lp_gcc.err" || true
  }
  if [[ -x "$TMPDIR/lp_spawn.bin" ]]; then
    out=$("$TMPDIR/lp_spawn.bin" 2>&1) || true
    if echo "$out" | grep -q 'spawn-residual-ok' \
      && echo "$out" | grep -q 'wait-residual-ok' \
      && echo "$out" | grep -q 'kill-residual-ok'; then
      pass "runtime residual Err sys_spawn/wait/kill"
    else
      bad "runtime residual out=$out"
    fi
    # forge deny: zero SysCap token
    sed -E 's/long long sys = oo_cap_grant_sys\(\)/long long sys = 0LL/' \
      "$TMPDIR/lp_spawn.c" >"$TMPDIR/lp_zero.c"
    gcc "${RT[@]}" "$TMPDIR/lp_zero.c" -o "$TMPDIR/lp_zero.bin" -lm -ldl -lpthread
    set +e
    zout=$("$TMPDIR/lp_zero.bin" 2>&1); zrc=$?
    set -e
    if [[ $zrc -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
      pass "zero Sys forge deny on sys_spawn"
    else
      bad "zero forge not denied out=$zout rc=$zrc"
    fi
  fi
fi

# Real blocking path: sys_exec under SysCap still works (product spawn+wait)
cat >"$TMPDIR/lp_exec.oo" <<'EOF'
pub fn main(sys: &SysCap) {
    let r = sys_exec(sys, "sh", "-c", "true");
    if r.is_ok() { println("exec-real-ok"); } else { println("exec-fail"); }
}
EOF
set +e
"$OODAC" emit-c "$TMPDIR/lp_exec.oo" >"$TMPDIR/lp_exec.c" 2>"$TMPDIR/lp_exec.err"
xerc=$?
set -e
if [[ $xerc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/lp_exec.c"; then
  bad "emit sys_exec contrast"
else
  gcc "${RT[@]}" "$TMPDIR/lp_exec.c" -o "$TMPDIR/lp_exec.bin" -lm -ldl -lpthread
  xout=$("$TMPDIR/lp_exec.bin" 2>&1) || true
  if echo "$xout" | grep -q 'exec-real-ok'; then
    pass "runtime sys_exec real blocking spawn+wait"
  else
    bad "sys_exec out=$xout"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "libfloor_process_smoke: FAILED" >&2
  exit 1
fi
echo "libfloor_process_smoke: PASSED"
exit 0
