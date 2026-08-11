#!/usr/bin/env bash
# M163 path A — joinable threads under ThreadCap
# Dual-run: grant path → join-ok; forge deny; bare refuse without ThreadCap
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
FIX="$ROOT/fixtures/thread_join.oo"

# check: bare thread_join without ThreadCap refused
cat >"$TMPDIR/tj_bare.oo" <<'EOF'
pub fn main() { let r = thread_join(0, 0); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/tj_bare.oo" >"$TMPDIR/tj_bare.out" 2>"$TMPDIR/tj_bare.err"
brc=$?
set -e
if [[ $brc -ne 0 ]] && grep -qiE 'capability|ThreadCap|ERR' "$TMPDIR/tj_bare.out" "$TMPDIR/tj_bare.err" 2>/dev/null; then
  pass "check refuse bare thread_join"
else
  bad "bare thread_join accepted rc=$brc"
  head -5 "$TMPDIR/tj_bare.out" "$TMPDIR/tj_bare.err" || true
fi

# check: bare thread_spawn without ThreadCap refused
cat >"$TMPDIR/tj_bare_sp.oo" <<'EOF'
pub fn main() { let r = thread_spawn("x"); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/tj_bare_sp.oo" >"$TMPDIR/tj_bare_sp.out" 2>"$TMPDIR/tj_bare_sp.err"
bsrc=$?
set -e
if [[ $bsrc -ne 0 ]] && grep -qiE 'capability|ThreadCap|ERR' "$TMPDIR/tj_bare_sp.out" "$TMPDIR/tj_bare_sp.err" 2>/dev/null; then
  pass "check refuse bare thread_spawn"
else
  bad "bare thread_spawn accepted rc=$bsrc"
fi

# check: granted ThreadCap fixture
set +e
"$OODAC_BIN" check "$FIX" >"$TMPDIR/tj_ck.out" 2>"$TMPDIR/tj_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/tj_ck.out"; then
  pass "check thread_join fixture with ThreadCap"
else
  bad "check fixture rc=$ckrc"
  head -10 "$TMPDIR/tj_ck.out" "$TMPDIR/tj_ck.err" || true
fi

# emit-c + grant path runtime
set +e
"$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/tj.c" 2>"$TMPDIR/tj.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/tj.c" "$TMPDIR/tj.err" 2>/dev/null; then
  bad "emit-c thread_join"
  head -15 "$TMPDIR/tj.err" "$TMPDIR/tj.c" || true
else
  pass "emit-c thread_join"
  if grep -qE 'oo_thread_spawn\(thread' "$TMPDIR/tj.c" \
    && grep -qE 'oo_thread_join\(thread' "$TMPDIR/tj.c" \
    && grep -q 'oo_cap_grant_thread' "$TMPDIR/tj.c"; then
    pass "emit lowers spawn+join + grant"
  else
    bad "emit missing oo_thread_spawn/join or grant"
    grep -nE 'thread_|grant' "$TMPDIR/tj.c" | head -20 || true
  fi
  gcc "${RT[@]}" "$TMPDIR/tj.c" -o "$TMPDIR/tj.bin" 2>"$TMPDIR/tj_gcc.err" || {
    bad "gcc thread_join"
    head -20 "$TMPDIR/tj_gcc.err" || true
  }
  if [[ -x "$TMPDIR/tj.bin" ]]; then
    out=$("$TMPDIR/tj.bin" 2>&1) || true
    if echo "$out" | grep -q 'join-ok'; then
      pass "runtime grant path join-ok"
    else
      bad "runtime grant out=$out"
    fi
  fi
  # forge deny: zero token
  if [[ -f "$TMPDIR/tj.c" ]]; then
    sed -E 's/long long thread = oo_cap_grant_thread\(\)/long long thread = 0LL/' \
      "$TMPDIR/tj.c" >"$TMPDIR/tj_zero.c"
    gcc "${RT[@]}" "$TMPDIR/tj_zero.c" -o "$TMPDIR/tj_zero.bin" -lm -ldl -lpthread 2>/dev/null || {
      bad "gcc zero forge"; true
    }
    if [[ -x "$TMPDIR/tj_zero.bin" ]]; then
      set +e
      zout=$("$TMPDIR/tj_zero.bin" 2>&1) || zrc=$?
      zrc=${zrc:-0}
      set -e
      if [[ ${zrc:-0} -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
        pass "zero forge deny"
      else
        bad "zero forge out=$zout rc=${zrc:-0}"
      fi
    fi
    # classic magic forge OOTH
    sed -E 's/long long thread = oo_cap_grant_thread\(\)/long long thread = 0x4F4F5448LL/' \
      "$TMPDIR/tj.c" >"$TMPDIR/tj_mag.c"
    gcc "${RT[@]}" "$TMPDIR/tj_mag.c" -o "$TMPDIR/tj_mag.bin" -lm -ldl -lpthread 2>/dev/null || {
      bad "gcc magic forge"; true
    }
    if [[ -x "$TMPDIR/tj_mag.bin" ]]; then
      set +e
      mout=$("$TMPDIR/tj_mag.bin" 2>&1) || mrc=$?
      mrc=${mrc:-0}
      set -e
      if [[ ${mrc:-0} -ne 0 ]] && echo "$mout" | grep -qE $'ERR[\t ]*cap'; then
        pass "magic forge deny"
      else
        bad "magic forge out=$mout rc=${mrc:-0}"
      fi
    fi
  fi
fi

# runtime honesty: joinable slot table (not detach-by-default)
if grep -q 'pthread_create' runtime/chs_rt_thread.c \
  && grep -q 'pthread_join' runtime/chs_rt_thread.c \
  && grep -q 'tid:%d' runtime/chs_rt_thread.c \
  && ! grep -q 'pthread_detach' runtime/chs_rt_thread.c; then
  pass "runtime joinable path A present (no detach default)"
else
  bad "runtime joinable path A missing"
fi
if grep -q 'pthread_mutex_lock' runtime/chs_rt_libfloor.c \
  && grep -q 'gpu residual: no device shaders' runtime/chs_rt_libfloor.c; then
  pass "mutex path A + GPU residual remain"
else
  bad "mutex/gpu path A broken"
fi

if [[ $fail -ne 0 ]]; then
  echo "thread_join_smoke: FAILED" >&2
  exit 1
fi
echo "thread_join_smoke: PASSED"
exit 0
