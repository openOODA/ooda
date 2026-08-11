#!/usr/bin/env bash
# M165 path A: simple arith + compare / ||+&& in requires (runtime emit)
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

build_run() {
  local src="$1" bin="$2"
  set +e
  OODAC_BIN="$OODAC_BIN" "$OODAC_BIN" build "$src" "$bin" >"$TMPDIR/carith_b.out" 2>"$TMPDIR/carith_b.err"
  local brc=$?
  set -e
  [[ $brc -eq 0 && -x "$bin" ]] || return 1
  return 0
}

# pass: x + 1 > 0 and (a>0||b>0)&&c>=0
if build_run "$ROOT/fixtures/contract_arith_pass.oo" "$TMPDIR/carith_pass"; then
  out=$("$TMPDIR/carith_pass" 2>&1) || true
  if echo "$out" | grep -q '1' && echo "$out" | grep -q '3'; then
    pass "arith requires pass (shifted=1, any_pos=3)"
  else
    bad "arith pass out=$out"
  fi
else
  bad "build contract_arith_pass (arith emit residual?)"
  head -15 "$TMPDIR/carith_b.err" || true
fi

# fail: x + 1 > 0 with x=-1
if build_run "$ROOT/fixtures/contract_arith_req_fail.oo" "$TMPDIR/carith_rfail"; then
  set +e
  out=$("$TMPDIR/carith_rfail" 2>&1); rc=$?
  set -e
  if [[ $rc -ne 0 ]] && echo "$out" | grep -qE 'contract|requires'; then
    pass "arith requires fail-closed"
  else
    bad "arith req fail out=$out rc=$rc"
  fi
else
  bad "build contract_arith_req_fail"
  head -10 "$TMPDIR/carith_b.err" || true
fi

# fail: (a>0||b>0)&&c>=0 with c=-1
if build_run "$ROOT/fixtures/contract_arith_or_and_fail.oo" "$TMPDIR/carith_oafail"; then
  set +e
  out=$("$TMPDIR/carith_oafail" 2>&1); rc=$?
  set -e
  if [[ $rc -ne 0 ]] && echo "$out" | grep -qE 'contract|requires'; then
    pass "||+&& requires fail-closed"
  else
    bad "or_and fail out=$out rc=$rc"
  fi
else
  bad "build contract_arith_or_and_fail"
  head -10 "$TMPDIR/carith_b.err" || true
fi

# honesty: residual markers still present (no full SMT claim)
if grep -q "CONTRACTS_COMPLEX_RESIDUAL_ALPHA" "$ROOT/bootstrap/CONTRACTS_COMPLEX.md" \
  && grep -qiE 'NO quantifier|no quantifier|not claim.*quantif|quantifiers' "$ROOT/bootstrap/CONTRACTS_COMPLEX.md" \
  && grep -qiE 'old-state|old state' "$ROOT/bootstrap/CONTRACTS_COMPLEX.md"; then
  pass "CONTRACTS_COMPLEX residual honesty (no quantifiers/old-state)"
else
  bad "CONTRACTS_COMPLEX honesty gaps"
fi

if [[ $fail -ne 0 ]]; then
  echo "contracts_arith_smoke: FAILED" >&2
  exit 1
fi
echo "contracts_arith_smoke: PASSED"
exit 0
