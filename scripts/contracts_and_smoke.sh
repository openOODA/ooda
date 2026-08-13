#!/usr/bin/env bash
# Simple && contracts product floor (Phase 1) — runtime pass/fail
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

build_run() {
  local src="$1" bin="$2"
  set +e
  OODAC_BIN="$OODAC_BIN" "$OODAC_BIN" build "$src" "$bin" >"$TMPDIR/cand_b.out" 2>"$TMPDIR/cand_b.err"
  local brc=$?
  set -e
  [[ $brc -eq 0 && -x "$bin" ]] || return 1
  return 0
}

# pass: && requires/ensures
if build_run "$ROOT/fixtures/complex_contract_pass.oo" "$TMPDIR/cand_pass"; then
  out=$("$TMPDIR/cand_pass" 2>&1) || true
  if echo "$out" | grep -q '7'; then
    pass "&& contract pass runs (7)"
  else
    bad "&& pass out=$out"
  fi
else
  bad "build complex_contract_pass"
  cat "$TMPDIR/cand_b.err" | head -10
fi

# fail requires
if build_run "$ROOT/fixtures/complex_contract_req_fail.oo" "$TMPDIR/cand_rfail"; then
  set +e
  out=$("$TMPDIR/cand_rfail" 2>&1); rc=$?
  set -e
  if [[ $rc -ne 0 ]] && echo "$out" | grep -qE 'contract|requires'; then
    pass "&& requires fail-closed"
  else
    bad "req fail out=$out rc=$rc"
  fi
else
  bad "build complex_contract_req_fail"
fi

# multi-clause still green
if bash "$ROOT/scripts/contracts_multi_clause_smoke.sh" >"$TMPDIR/cand_mc.log" 2>&1; then
  pass "multi_clause still green"
else
  bad "multi_clause"
  tail -8 "$TMPDIR/cand_mc.log" || true
fi

if [[ $fail -ne 0 ]]; then
  echo "contracts_and_smoke: FAILED" >&2
  exit 1
fi
echo "contracts_and_smoke: PASSED"
exit 0
