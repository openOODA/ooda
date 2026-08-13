#!/usr/bin/env bash
# M166 path A — sealed OS syscall free names under SysCap (residual Err)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
FIX="$ROOT/fixtures/sys_syscall_path_a.oo"

# Source: seal table + lower + runtime residual
if grep -q 'sys_epoll_create' oodac/check_cap_util.oo \
  && grep -q 'sys_inotify_init' oodac/check_cap_util.oo \
  && grep -q 'sys_prctl' oodac/check_cap_util.oo; then
  pass "is_sealed_sys OS syscall names"
else
  bad "check_cap_util missing sys_epoll/inotify/prctl"
fi
if grep -q 'oo_sys_epoll_create' oodac/c_emit_sys.oo \
  && grep -q 'oo_sys_inotify_init' oodac/c_emit_sys.oo \
  && grep -q 'oo_sys_prctl' oodac/c_emit_sys.oo; then
  pass "c_emit_sys lowers"
else
  bad "c_emit_sys missing lowers"
fi
if grep -q 'not full async I/O' runtime/chs_rt_libfloor.c \
  && grep -q 'oo_sys_epoll_create' runtime/chs_rt_libfloor.c \
  && grep -q 'oo_sys_inotify_init' runtime/chs_rt_libfloor.c \
  && grep -q 'oo_sys_prctl' runtime/chs_rt_libfloor.c; then
  pass "runtime residual Err after SysCap require"
else
  bad "runtime missing residual stubs"
fi
if grep -q 'process_epoll_create' std/os/process.oo \
  && grep -q '&SysCap' std/os/process.oo; then
  pass "std/os/process.oo SysCap wrappers"
else
  bad "process.oo missing wrappers"
fi
# M166 std cap scoping samples path A — bootstrap honesty (AGY)
if grep -q 'M166 path A — std cap scoping samples' bootstrap/STATIC_CAPS.oot \
  && grep -q 'as fn' bootstrap/STATIC_CAPS.oot \
  && grep -q '&NetCap' std/os/net.oo; then
  pass "STATIC_CAPS std cap scoping + as-fn residual; net.oo NetCap"
else
  bad "missing STATIC_CAPS M166 std cap scoping note or net.oo NetCap"
fi

[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: $OODAC" >&2; exit 1; }

# bare refuse without SysCap
cat >"$TMPDIR/sys_bare.oo" <<'EOF'
pub fn main() { let r = sys_epoll_create(0); }
EOF
set +e
"$OODAC" check "$TMPDIR/sys_bare.oo" >"$TMPDIR/sys_bare.out" 2>"$TMPDIR/sys_bare.err"
brc=$?
set -e
if [[ $brc -ne 0 ]] && grep -qiE 'capability|SysCap|ERR' "$TMPDIR/sys_bare.out" "$TMPDIR/sys_bare.err" 2>/dev/null; then
  pass "check refuse bare sys_epoll_create"
else
  bad "bare sys_epoll_create accepted rc=$brc (rebuild oodac if seal lag)"
  head -5 "$TMPDIR/sys_bare.out" "$TMPDIR/sys_bare.err" || true
fi

set +e
"$OODAC" check "$FIX" >"$TMPDIR/sys_ck.out" 2>"$TMPDIR/sys_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/sys_ck.out"; then
  pass "check sys_syscall_path_a with SysCap"
else
  bad "check fixture rc=$ckrc"
  head -10 "$TMPDIR/sys_ck.out" "$TMPDIR/sys_ck.err" 2>/dev/null || true
fi

set +e
"$OODAC" emit-c "$FIX" >"$TMPDIR/sys.c" 2>"$TMPDIR/sys.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/sys.c" "$TMPDIR/sys.err" 2>/dev/null; then
  bad "emit-c sys_syscall_path_a"
  head -15 "$TMPDIR/sys.err" "$TMPDIR/sys.c" 2>/dev/null || true
else
  if grep -qE 'oo_sys_epoll_create\(sys' "$TMPDIR/sys.c" \
    && grep -qE 'oo_sys_inotify_init\(sys' "$TMPDIR/sys.c" \
    && grep -qE 'oo_sys_prctl\(sys' "$TMPDIR/sys.c" \
    && grep -q 'oo_cap_grant_sys' "$TMPDIR/sys.c"; then
    pass "emit lowers + SysCap grant"
  else
    bad "emit missing oo_sys_* or grant"
    grep -nE 'sys_|grant' "$TMPDIR/sys.c" | head -20 || true
  fi
  gcc "${RT[@]}" "$TMPDIR/sys.c" -o "$TMPDIR/sys.bin" 2>"$TMPDIR/sys.gcc" || {
    bad "gcc sys_syscall"; head -15 "$TMPDIR/sys.gcc" || true
  }
  if [[ -x "$TMPDIR/sys.bin" ]]; then
    out=$("$TMPDIR/sys.bin" 2>&1) || true
    if echo "$out" | grep -q 'epoll-residual-ok' \
      && echo "$out" | grep -q 'inotify-residual-ok' \
      && echo "$out" | grep -q 'prctl-residual-ok'; then
      pass "runtime residual Err path"
    else
      bad "runtime out=$out"
    fi
    # forge zero deny
    sed -E 's/long long sys = oo_cap_grant_sys\(\)/long long sys = 0LL/' \
      "$TMPDIR/sys.c" >"$TMPDIR/sys_z.c"
    gcc "${RT[@]}" "$TMPDIR/sys_z.c" -o "$TMPDIR/sys_z.bin" 2>/dev/null || true
    if [[ -x "$TMPDIR/sys_z.bin" ]]; then
      set +e
      zout=$("$TMPDIR/sys_z.bin" 2>&1) || zrc=$?
      zrc=${zrc:-0}
      set -e
      if [[ $zrc -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
        pass "SysCap forge zero deny"
      else
        bad "forge not denied out=$zout rc=$zrc"
      fi
    fi
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "sys_syscall_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "sys_syscall_path_a_smoke: PASSED"
exit 0
