#!/usr/bin/env bash
# job: M11 product run --engine bc|native parity + fail-closed flags
# in:  bin/ooda, oodac/oodac; fixtures that both engines support
# out: exit 0 if ≥3 fixtures match native≡bc≡expect and bad engines fail
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: $OODA" >&2
  exit 1
fi
if [[ ! -x "$OODAC_BIN" ]]; then
  echo "ERR_NO_OODAC: $OODAC_BIN" >&2
  exit 1
fi

run_one() {
  local engine="$1" src="$2"
  set +e
  timeout 30 "$OODA" run --engine "$engine" "$src" 2>"$TMPDIR/re_${engine}.err"
  local rc=$?
  set -e
  return $rc
}

# Fail-closed flags
set +e
"$OODA" run --engine bogon "$ROOT/fixtures/chs_hello.oo" >"$TMPDIR/re_bad.out" 2>"$TMPDIR/re_bad.err"
bad_rc=$?
set -e
if [[ $bad_rc -ne 0 ]] && grep -qiE 'invalid engine|ERR' "$TMPDIR/re_bad.out" "$TMPDIR/re_bad.err" 2>/dev/null; then
  pass "run --engine bogon fail-closed"
else
  bad "run --engine bogon should fail-closed rc=$bad_rc"
fi

set +e
"$OODA" run --engine >"$TMPDIR/re_miss.out" 2>"$TMPDIR/re_miss.err"
miss_rc=$?
set -e
if [[ $miss_rc -ne 0 ]]; then
  pass "run --engine (no value) fail-closed"
else
  bad "run --engine missing value should fail"
fi

parity() {
  local name="$1" src="$2" expect="$3"
  local nout bout
  set +e
  nout=$(timeout 30 "$OODA" run --engine native "$src" 2>"$TMPDIR/re_n_${name}.err")
  nrc=$?
  bout=$(timeout 30 "$OODA" run --engine bc "$src" 2>"$TMPDIR/re_b_${name}.err")
  brc=$?
  set -e
  if [[ $nrc -ne 0 ]]; then
    bad "native $name rc=$nrc"
    cat "$TMPDIR/re_n_${name}.err" | head -8 || true
    return
  fi
  if [[ $brc -ne 0 ]]; then
    bad "bc $name rc=$brc"
    cat "$TMPDIR/re_b_${name}.err" | head -8 || true
    return
  fi
  if [[ "$nout" != "$expect" ]]; then
    bad "native $name want $(printf %q "$expect") got $(printf %q "$nout")"
    return
  fi
  if [[ "$bout" != "$expect" ]]; then
    bad "bc $name want $(printf %q "$expect") got $(printf %q "$bout")"
    return
  fi
  if [[ "$nout" != "$bout" ]]; then
    bad "parity $name native≠bc"
    return
  fi
  # default run ≡ native
  set +e
  dout=$(timeout 30 "$OODA" run "$src" 2>"$TMPDIR/re_d_${name}.err")
  drc=$?
  set -e
  if [[ $drc -ne 0 || "$dout" != "$nout" ]]; then
    bad "default run $name not native (rc=$drc out=$(printf %q "$dout"))"
    return
  fi
  pass "parity $name native≡bc≡default"
}

n=0
parity "chs_hello" "$ROOT/fixtures/chs_hello.oo" "1"
n=$((n + 1))
parity "while_count" "$ROOT/fixtures/while_count.oo" "3"
n=$((n + 1))
parity "for_range" "$ROOT/fixtures/for_range.oo" "10"
n=$((n + 1))

if [[ $fail -ne 0 ]]; then
  echo "run_engine_parity_smoke: FAILED" >&2
  exit 1
fi
if [[ $n -lt 3 ]]; then
  echo "run_engine_parity_smoke: need ≥3 fixtures" >&2
  exit 1
fi
echo "run_engine_parity_smoke: PASSED ($n fixtures; default=native)"
exit 0
