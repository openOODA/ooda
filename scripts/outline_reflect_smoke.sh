#!/usr/bin/env bash
# job: rails for ooda outline + ooda reflect (parse-only agent tools)
# in:  bin/ooda (or OODA); fixtures/outline_reflect_pass.oo
# out: exit 0 if pass+fail rails green
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
FIX="$ROOT/fixtures/outline_reflect_pass.oo"
PY="$ROOT/scripts/ooda_outline_reflect.py"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: $OODA" >&2
  exit 1
fi
if [[ ! -f "$PY" ]]; then
  echo "ERR_NO_HELPER: $PY" >&2
  exit 1
fi
if [[ ! -f "$FIX" ]]; then
  echo "ERR_NO_FIX: $FIX" >&2
  exit 1
fi

# --- outline pass: pub only, caps, no private ---
set +e
"$OODA" outline "$FIX" >"$TMPDIR/or_outline.out" 2>"$TMPDIR/or_outline.err"
ro=$?
set -e
if [[ $ro -ne 0 ]]; then
  bad "outline pass exit=$ro"
  cat "$TMPDIR/or_outline.err" >&2 || true
else
  if ! grep -q 'pub fn scale(x: Int) -> Int' "$TMPDIR/or_outline.out"; then
    bad "outline missing scale"
  elif ! grep -q 'pub fn touch(fs: &FsCap, path: String) -> String caps=FsCap' "$TMPDIR/or_outline.out"; then
    bad "outline missing touch caps"
  elif ! grep -q 'pub fn main(sys: &SysCap) caps=SysCap' "$TMPDIR/or_outline.out"; then
    bad "outline missing main caps"
  elif grep -q 'hidden' "$TMPDIR/or_outline.out"; then
    bad "outline leaked private fn"
  elif grep -q 'requires\|ensures\|verify' "$TMPDIR/or_outline.out"; then
    bad "outline not token-cheap (contracts/verify present)"
  else
    pass "outline pass compact pub+caps"
  fi
fi

# --- outline fail: unreadable ---
set +e
"$OODA" outline "$TMPDIR/no_such_outline_$$.oo" >"$TMPDIR/or_ol_miss.out" 2>"$TMPDIR/or_ol_miss.err"
rm=$?
set -e
if [[ $rm -eq 0 ]]; then
  bad "outline accepted missing file"
elif ! grep -qE $'^ERR\toutline\t' "$TMPDIR/or_ol_miss.err" "$TMPDIR/or_ol_miss.out" 2>/dev/null; then
  # product may surface via stderr only
  if grep -q 'unreadable\|missing' "$TMPDIR/or_ol_miss.err" "$TMPDIR/or_ol_miss.out" 2>/dev/null; then
    pass "outline fail-closed missing file"
  else
    bad "outline missing-file no ERR (exit=$rm)"
    head -5 "$TMPDIR/or_ol_miss.err" "$TMPDIR/or_ol_miss.out" >&2 || true
  fi
else
  pass "outline fail-closed missing file"
fi

# --- reflect pass: contracts + caps + verify ---
set +e
"$OODA" reflect "$FIX" >"$TMPDIR/or_reflect.out" 2>"$TMPDIR/or_reflect.err"
rr=$?
set -e
if [[ $rr -ne 0 ]]; then
  bad "reflect pass exit=$rr"
  cat "$TMPDIR/or_reflect.err" >&2 || true
else
  if ! grep -q '"kind":"fn"' "$TMPDIR/or_reflect.out"; then
    bad "reflect missing fn lines"
  elif ! grep -q '"name":"scale"' "$TMPDIR/or_reflect.out"; then
    bad "reflect missing scale"
  elif ! grep -q 'x >= 0' "$TMPDIR/or_reflect.out"; then
    bad "reflect missing requires text"
  elif ! grep -q 'result >= x' "$TMPDIR/or_reflect.out"; then
    bad "reflect missing ensures text"
  elif ! grep -q '"caps":\["FsCap"\]' "$TMPDIR/or_reflect.out"; then
    bad "reflect missing FsCap"
  elif ! grep -q '"kind":"verify","name":"scale"' "$TMPDIR/or_reflect.out"; then
    bad "reflect missing verify scale"
  elif ! grep -q '"name":"hidden"' "$TMPDIR/or_reflect.out"; then
    bad "reflect should include private fn"
  else
    pass "reflect pass contracts+caps+verify"
  fi
fi

# --- reflect symbol filter ---
set +e
"$OODA" reflect "$FIX" scale >"$TMPDIR/or_sym.out" 2>"$TMPDIR/or_sym.err"
rs=$?
set -e
if [[ $rs -ne 0 ]]; then
  bad "reflect symbol scale exit=$rs"
elif ! grep -q '"name":"scale"' "$TMPDIR/or_sym.out"; then
  bad "reflect symbol filter missing scale"
elif grep -q '"name":"main"' "$TMPDIR/or_sym.out"; then
  bad "reflect symbol filter leaked main"
else
  pass "reflect symbol filter"
fi

# --- reflect fail: bad symbol ---
set +e
"$OODA" reflect "$FIX" no_such_symbol_xyz >"$TMPDIR/or_badsym.out" 2>"$TMPDIR/or_badsym.err"
rb=$?
set -e
if [[ $rb -eq 0 ]]; then
  bad "reflect accepted unknown symbol"
else
  pass "reflect fail-closed unknown symbol"
fi

# --- reflect fail: unreadable ---
set +e
"$OODA" reflect /no/such/reflect_$$.oo >"$TMPDIR/or_rf_miss.out" 2>"$TMPDIR/or_rf_miss.err"
rx=$?
set -e
if [[ $rx -eq 0 ]]; then
  bad "reflect accepted missing file"
else
  pass "reflect fail-closed missing file"
fi

# --- security: helper must not invoke oodac build/run ---
if grep -nE 'oodac|subprocess|os\.system|Popen' "$PY" | grep -vE 'outline_reflect|test_harness|#' >/dev/null 2>&1; then
  # allow only comments / docstrings mentioning oodac as forbidden
  if grep -nE 'subprocess|os\.system|Popen|oodac build|oodac run' "$PY" | grep -vE 'never|not |No |#|"""' >/dev/null 2>&1; then
    bad "helper may execute user code"
  else
    pass "helper parse-only (no exec APIs)"
  fi
else
  pass "helper parse-only (no exec APIs)"
fi

if [[ $fail -ne 0 ]]; then
  echo "outline_reflect_smoke: FAILED" >&2
  exit 1
fi
echo "outline_reflect_smoke: PASSED"
exit 0
