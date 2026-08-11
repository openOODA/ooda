#!/usr/bin/env bash
# M165 path A — thin actors under ThreadCap (spawn + mailbox send/recv)
# Dual-run: bare refuse; grant path → ping; forge deny; not residual free-name
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
FIX="$ROOT/fixtures/actor_path_a.oo"

# residual: actor_spawn not free-name refuse
if ! grep -qE 'name == "actor_spawn"' oodac/check_residual.oo; then
  pass "actor_spawn not residual refuse"
else
  bad "actor_spawn still residual refuse"
fi

# check: bare actor_spawn without ThreadCap refused (capability)
cat >"$TMPDIR/act_bare.oo" <<'EOF'
pub fn main() { let r = actor_spawn("x"); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/act_bare.oo" >"$TMPDIR/act_bare.out" 2>"$TMPDIR/act_bare.err"
brc=$?
set -e
if [[ $brc -ne 0 ]] && grep -qiE 'capability|ThreadCap|ERR' "$TMPDIR/act_bare.out" "$TMPDIR/act_bare.err" 2>/dev/null; then
  pass "check refuse bare actor_spawn"
else
  bad "bare actor_spawn accepted rc=$brc"
  head -5 "$TMPDIR/act_bare.out" "$TMPDIR/act_bare.err" || true
fi

# check: granted ThreadCap fixture
set +e
"$OODAC_BIN" check "$FIX" >"$TMPDIR/act_ck.out" 2>"$TMPDIR/act_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/act_ck.out"; then
  pass "check actor_path_a with ThreadCap"
else
  bad "check fixture rc=$ckrc"
  head -10 "$TMPDIR/act_ck.out" "$TMPDIR/act_ck.err" || true
fi

# emit-c + grant path runtime
set +e
"$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/act.c" 2>"$TMPDIR/act.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/act.c" "$TMPDIR/act.err" 2>/dev/null; then
  bad "emit-c actor_path_a"
  head -15 "$TMPDIR/act.err" "$TMPDIR/act.c" || true
else
  pass "emit-c actor_path_a"
  if grep -qE 'oo_actor_spawn\(thread' "$TMPDIR/act.c" \
    && grep -qE 'oo_actor_send\(thread' "$TMPDIR/act.c" \
    && grep -qE 'oo_actor_recv\(thread' "$TMPDIR/act.c" \
    && grep -q 'oo_cap_grant_thread' "$TMPDIR/act.c"; then
    pass "emit lowers actor_spawn/send/recv + grant"
  else
    bad "emit missing oo_actor_* or grant"
    grep -nE 'actor_|grant' "$TMPDIR/act.c" | head -20 || true
  fi
  gcc "${RT[@]}" "$TMPDIR/act.c" -o "$TMPDIR/act.bin" 2>"$TMPDIR/act_gcc.err" || {
    bad "gcc actor_path_a"
    head -20 "$TMPDIR/act_gcc.err" || true
  }
  if [[ -x "$TMPDIR/act.bin" ]]; then
    out=$("$TMPDIR/act.bin" 2>&1) || true
    if echo "$out" | grep -qx 'ping'; then
      pass "runtime grant path ping"
    else
      bad "runtime grant out=$out"
    fi
  fi
  # forge deny: zero token
  if [[ -f "$TMPDIR/act.c" ]]; then
    sed -E 's/long long thread = oo_cap_grant_thread\(\)/long long thread = 0LL/' \
      "$TMPDIR/act.c" >"$TMPDIR/act_zero.c"
    gcc "${RT[@]}" "$TMPDIR/act_zero.c" -o "$TMPDIR/act_zero.bin" 2>/dev/null || {
      bad "gcc zero forge"; true
    }
    if [[ -x "$TMPDIR/act_zero.bin" ]]; then
      set +e
      zout=$("$TMPDIR/act_zero.bin" 2>&1) || zrc=$?
      zrc=${zrc:-0}
      set -e
      if [[ ${zrc:-0} -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
        pass "zero forge deny"
      else
        bad "zero forge out=$zout rc=${zrc:-0}"
      fi
    fi
  fi
fi

# runtime honesty
if grep -q 'OO_ACTOR_SLOTS' runtime/chs_rt_actor.c \
  && grep -q 'actor:%d' runtime/chs_rt_actor.c \
  && grep -q 'pthread_create' runtime/chs_rt_actor.c \
  && grep -q 'oo_actor_send' runtime/chs_rt_actor.c; then
  pass "runtime actor path A present"
else
  bad "runtime actor path A missing"
fi
if grep -q 'chs_rt_actor.c' runtime/chs_rt.c; then
  pass "chs_rt.c includes actor"
else
  bad "chs_rt.c missing actor include"
fi
if grep -q 'actor_spawn' oodac/check_cap_util.oo \
  && grep -q 'is_sealed_thread' oodac/check_cap_util.oo; then
  pass "actor sealed under ThreadCap"
else
  bad "seal table missing actor_*"
fi

if [[ $fail -ne 0 ]]; then
  echo "actor_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "actor_path_a_smoke: PASSED"
exit 0
