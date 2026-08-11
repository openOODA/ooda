#!/usr/bin/env bash
# M164 path A — process-local channels under ThreadCap
# Dual-run: bare refuse; grant path → hi; forge deny; actor residual
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
FIX="$ROOT/fixtures/channel_roundtrip.oo"

# check: bare channel_new without ThreadCap refused (capability, not residual)
cat >"$TMPDIR/ch_bare.oo" <<'EOF'
pub fn main() { let r = channel_new(); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/ch_bare.oo" >"$TMPDIR/ch_bare.out" 2>"$TMPDIR/ch_bare.err"
brc=$?
set -e
if [[ $brc -ne 0 ]] && grep -qiE 'capability|ThreadCap|ERR' "$TMPDIR/ch_bare.out" "$TMPDIR/ch_bare.err" 2>/dev/null; then
  pass "check refuse bare channel_new"
else
  bad "bare channel_new accepted rc=$brc"
  head -5 "$TMPDIR/ch_bare.out" "$TMPDIR/ch_bare.err" || true
fi

# residual: actor_spawn still CONCURRENCY refuse
cat >"$TMPDIR/ch_actor.oo" <<'EOF'
pub fn main() { let r = actor_spawn(); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/ch_actor.oo" >"$TMPDIR/ch_actor.out" 2>"$TMPDIR/ch_actor.err"
arc=$?
set -e
if [[ $arc -ne 0 ]] && grep -qE $'ERR\tresidual|Residual product refuse' "$TMPDIR/ch_actor.out" "$TMPDIR/ch_actor.err" 2>/dev/null; then
  pass "check residual refuse actor_spawn"
else
  bad "actor_spawn residual refuse missing rc=$arc"
fi

# check: granted ThreadCap fixture
set +e
"$OODAC_BIN" check "$FIX" >"$TMPDIR/ch_ck.out" 2>"$TMPDIR/ch_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/ch_ck.out"; then
  pass "check channel_roundtrip with ThreadCap"
else
  bad "check fixture rc=$ckrc"
  head -10 "$TMPDIR/ch_ck.out" "$TMPDIR/ch_ck.err" || true
fi

# emit-c + grant path runtime
set +e
"$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/ch.c" 2>"$TMPDIR/ch.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/ch.c" "$TMPDIR/ch.err" 2>/dev/null; then
  bad "emit-c channel_roundtrip"
  head -15 "$TMPDIR/ch.err" "$TMPDIR/ch.c" || true
else
  pass "emit-c channel_roundtrip"
  if grep -qE 'oo_channel_new\(thread' "$TMPDIR/ch.c" \
    && grep -qE 'oo_channel_send\(thread' "$TMPDIR/ch.c" \
    && grep -qE 'oo_channel_recv\(thread' "$TMPDIR/ch.c" \
    && grep -q 'oo_cap_grant_thread' "$TMPDIR/ch.c"; then
    pass "emit lowers channel_new/send/recv + grant"
  else
    bad "emit missing oo_channel_* or grant"
    grep -nE 'channel_|grant' "$TMPDIR/ch.c" | head -20 || true
  fi
  gcc "${RT[@]}" "$TMPDIR/ch.c" -o "$TMPDIR/ch.bin" 2>"$TMPDIR/ch_gcc.err" || {
    bad "gcc channel_roundtrip"
    head -20 "$TMPDIR/ch_gcc.err" || true
  }
  if [[ -x "$TMPDIR/ch.bin" ]]; then
    out=$("$TMPDIR/ch.bin" 2>&1) || true
    if echo "$out" | grep -qx 'hi'; then
      pass "runtime grant path hi"
    else
      bad "runtime grant out=$out"
    fi
  fi
  # forge deny: zero token
  if [[ -f "$TMPDIR/ch.c" ]]; then
    sed -E 's/long long thread = oo_cap_grant_thread\(\)/long long thread = 0LL/' \
      "$TMPDIR/ch.c" >"$TMPDIR/ch_zero.c"
    gcc "${RT[@]}" "$TMPDIR/ch_zero.c" -o "$TMPDIR/ch_zero.bin" 2>/dev/null || {
      bad "gcc zero forge"; true
    }
    if [[ -x "$TMPDIR/ch_zero.bin" ]]; then
      set +e
      zout=$("$TMPDIR/ch_zero.bin" 2>&1) || zrc=$?
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
      "$TMPDIR/ch.c" >"$TMPDIR/ch_mag.c"
    gcc "${RT[@]}" "$TMPDIR/ch_mag.c" -o "$TMPDIR/ch_mag.bin" 2>/dev/null || {
      bad "gcc magic forge"; true
    }
    if [[ -x "$TMPDIR/ch_mag.bin" ]]; then
      set +e
      mout=$("$TMPDIR/ch_mag.bin" 2>&1) || mrc=$?
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

# runtime honesty: bounded queue + mutex present
if grep -q 'OO_CH_SLOTS' runtime/chs_rt_channel.c \
  && grep -q 'pthread_mutex_lock' runtime/chs_rt_channel.c \
  && grep -q 'pthread_cond_' runtime/chs_rt_channel.c \
  && grep -q 'ch:%d' runtime/chs_rt_channel.c; then
  pass "runtime channel path A present"
else
  bad "runtime channel path A missing"
fi
if grep -q 'chs_rt_channel.c' runtime/chs_rt.c; then
  pass "chs_rt.c includes channel"
else
  bad "chs_rt.c missing channel include"
fi
# residual: channel_* not residual free-name; actor still is
if ! grep -qE 'name == "channel_' oodac/check_residual.oo \
  && grep -qE 'name == "actor_spawn"' oodac/check_residual.oo; then
  pass "residual honesty: channels out, actor residual"
else
  bad "residual table wrong for channels/actors"
fi

if [[ $fail -ne 0 ]]; then
  echo "channel_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "channel_path_a_smoke: PASSED"
exit 0
