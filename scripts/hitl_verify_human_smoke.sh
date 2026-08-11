#!/usr/bin/env bash
# M165 path A — verify_human free builtin (env-gated Result, not residual refuse)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
FIX="$ROOT/fixtures/hitl_verify_human.oo"

# residual table must not refuse verify_human
if ! grep -qE 'name == "verify_human"' oodac/check_residual.oo; then
  pass "verify_human not residual refuse"
else
  bad "verify_human still residual refuse"
fi

# check accepts free call (known builtin)
set +e
"$OODAC_BIN" check "$FIX" >"$TMPDIR/vh_ck.out" 2>"$TMPDIR/vh_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/vh_ck.out"; then
  pass "check verify_human free name"
else
  bad "check verify_human rc=$ckrc"
  head -10 "$TMPDIR/vh_ck.out" "$TMPDIR/vh_ck.err" || true
fi

# emit-c lowers to oo_verify_human
set +e
"$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/vh.c" 2>"$TMPDIR/vh.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/vh.c" "$TMPDIR/vh.err" 2>/dev/null; then
  bad "emit-c verify_human"
  head -15 "$TMPDIR/vh.err" "$TMPDIR/vh.c" || true
else
  pass "emit-c verify_human"
  if grep -q 'oo_verify_human' "$TMPDIR/vh.c"; then
    pass "emit lowers oo_verify_human"
  else
    bad "emit missing oo_verify_human"
  fi
  gcc "${RT[@]}" "$TMPDIR/vh.c" -o "$TMPDIR/vh.bin" 2>"$TMPDIR/vh_gcc.err" || {
    bad "gcc verify_human"
    head -20 "$TMPDIR/vh_gcc.err" || true
  }
fi

if [[ -x "$TMPDIR/vh.bin" ]]; then
  # deny without OODA_HITL_ALLOW → Err path (fixture prints "denied")
  set +e
  out=$("$TMPDIR/vh.bin" 2>"$TMPDIR/vh_deny.err")
  drc=$?
  set -e
  if echo "$out" | grep -qx 'denied'; then
    pass "runtime deny without allow (Result Err → denied)"
  else
    bad "deny without allow out=$out"
    head -5 "$TMPDIR/vh_deny.err" || true
  fi

  # auto-approve path A
  set +e
  out=$(OODA_HITL_ALLOW=1 OODA_HITL_AUTO_APPROVE=1 "$TMPDIR/vh.bin" 2>"$TMPDIR/vh_ok.err")
  arc=$?
  set -e
  if echo "$out" | grep -qx 'approved'; then
    pass "runtime auto-approve Ok → approved"
  else
    bad "auto-approve out=$out rc=$arc"
    head -8 "$TMPDIR/vh_ok.err" || true
  fi
  if grep -qiE 'Auto-approved|verify_human' "$TMPDIR/vh_ok.err" 2>/dev/null; then
    pass "auto-approve logs HITL"
  else
    bad "missing HITL log on auto-approve"
  fi
fi

# runtime + preamble honesty
if grep -q 'oo_verify_human' runtime/chs_rt_hitl.c \
  && grep -q 'OODA_HITL_ALLOW' runtime/chs_rt_hitl.c \
  && grep -q 'OODA_HITL_AUTO_APPROVE' runtime/chs_rt_hitl.c; then
  pass "runtime hitl path A present"
else
  bad "runtime hitl missing"
fi
if grep -q 'chs_rt_hitl.c' runtime/chs_rt.c; then
  pass "chs_rt.c includes hitl"
else
  bad "chs_rt.c missing hitl include"
fi
if grep -q 'oo_verify_human' oodac/c_emit_preamble.oo \
  && grep -q 'verify_human' oodac/tc_names.oo; then
  pass "preamble + tc_names wire"
else
  bad "preamble/tc_names missing verify_human"
fi

if [[ $fail -ne 0 ]]; then
  echo "hitl_verify_human_smoke: FAILED" >&2
  exit 1
fi
echo "hitl_verify_human_smoke: PASSED"
exit 0
