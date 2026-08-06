#!/usr/bin/env bash
# job: alternate problem-search rails — differential, mutation, honesty traps
# in:  oodac + product ooda + fixtures
# out: exit 0 if known lies stay fixed; non-zero on regression
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR/ph"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
OODA="${OODA:-$ROOT/bin/ooda}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
[[ -x "$OODA" ]] || { echo "ERR_NO_OODA" >&2; exit 1; }

# --- 1) emit-c multi-file must expand (not incomplete helper) ---
"$OODAC" emit-c "$ROOT/bootstrap/corpus/import/pass/multi_ok.oo" \
  >"$TMPDIR/ph/multi.c" 2>"$TMPDIR/ph/multi.err" || true
if grep -q 'helper(' "$TMPDIR/ph/multi.c" && grep -qE 'long long helper|helper\(void\)' "$TMPDIR/ph/multi.c"; then
  pass "emit-c multi-file expands helper"
else
  # allow pure multi path: build still works
  if grep -q 'helper()' "$TMPDIR/ph/multi.c" && ! grep -qE 'helper\(void\)|long long helper' "$TMPDIR/ph/multi.c"; then
    bad "emit-c multi_ok incomplete (helper used, not defined)"
  else
    bad "emit-c multi_ok unexpected output"
  fi
fi

# --- 2) simple requires runtime enforced ---
cat >"$TMPDIR/ph/req.oo" <<'EOF'
pub fn scale(x: Int) -> Int
    requires x >= 0
{
    return x + 1;
}
pub fn main() {
    println(scale(0 - 1));
}
EOF
set +e
"$OODAC" build "$TMPDIR/ph/req.oo" "$TMPDIR/ph/req" >"$TMPDIR/ph/req_b.out" 2>"$TMPDIR/ph/req_b.err"
br=$?
set -e
if [[ $br -ne 0 || ! -x "$TMPDIR/ph/req" ]]; then
  bad "requires fixture failed to build"
else
  set +e
  out=$("$TMPDIR/ph/req" 2>&1)
  rr=$?
  set -e
  if [[ $rr -eq 0 ]]; then
    bad "requires x>=0 did not trap scale(-1) (got: $out)"
  elif echo "$out" | grep -q 'contract'; then
    pass "requires runtime traps violation"
  else
    bad "requires trap missing contract message (exit=$rr out=$out)"
  fi
fi

# --- 3) assert_ne! in verify ---
cat >"$TMPDIR/ph/vne.oo" <<'EOF'
pub fn add(a: Int, b: Int) -> Int { return a + b; }
verify add {
    assert_ne!(add(1, 1), 3);
    assert!(add(2, 2) == 4);
}
pub fn main() { println(1); }
EOF
set +e
"$OODA" test "$TMPDIR/ph/vne.oo" >"$TMPDIR/ph/vne.out" 2>"$TMPDIR/ph/vne.err"
vt=$?
set -e
if [[ $vt -ne 0 ]] || ! grep -q 'OK verify' "$TMPDIR/ph/vne.out"; then
  bad "assert_ne/assert! verify (exit=$vt)"
  cat "$TMPDIR/ph/vne.out" "$TMPDIR/ph/vne.err" | head -15 || true
else
  pass "assert_ne + assert! verify"
fi

# --- 4) free-call cap still sealed ---
set +e
"$OODAC" check "$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo" >/dev/null 2>&1
rc=$?
set -e
if [[ $rc -eq 0 ]]; then bad "cap free-call soft-pass"; else pass "cap free-call fail-closed"; fi

# --- 5) product json-errors not residual-soft ---
set +e
"$OODA" check --json-errors "$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo" \
  >"$TMPDIR/ph/j.out" 2>"$TMPDIR/ph/j.err"
jr=$?
set -e
if [[ $jr -eq 0 ]]; then bad "json-errors cap exit 0"; 
elif grep -q E_CAP "$TMPDIR/ph/j.out"; then pass "json-errors E_CAP";
else bad "json-errors missing E_CAP"; fi

# --- 6) no OK_HOST in pure sources ---
if grep -rq 'OK_HOST' "$ROOT/oodac" "$ROOT/cli" --include='*.oo' 2>/dev/null; then
  bad "OK_HOST in pure sources"
else
  pass "no OK_HOST"
fi

# --- 7) div-by-zero initializer fail-closed ---
cat >"$TMPDIR/ph/div.oo" <<'EOF'
pub fn main() {
    let x = 1 / 0;
    println(x);
}
EOF
set +e
"$OODAC" check "$TMPDIR/ph/div.oo" >/dev/null 2>&1
dc=$?
set -e
if [[ $dc -eq 0 ]]; then bad "div-by-zero soft-pass"; else pass "div-by-zero fail-closed"; fi

# --- 8) cap seal: param name FsCap must not grant ---
set +e
"$OODAC" check "$ROOT/bootstrap/corpus/check/fail/cap_name_not_type.oo" >/dev/null 2>&1
cn=$?
set -e
if [[ $cn -eq 0 ]]; then bad "cap name bypass soft-pass"; else pass "cap name not type fail-closed"; fi
set +e
"$OODAC" check "$ROOT/bootstrap/corpus/check/fail/list_fscap_not_grant.oo" >/dev/null 2>&1
cl=$?
set -e
if [[ $cl -eq 0 ]]; then bad "List[FsCap] grant soft-pass"; else pass "List[FsCap] not grant"; fi

# --- 9) shell injection via product CLI path must not run attacker cmd ---
rm -f /tmp/ooda_inject_marker_ph
set +e
"$OODA" check 'fixtures/int_main.oo"; touch /tmp/ooda_inject_marker_ph; echo "' \
  >"$TMPDIR/ph/inj.out" 2>"$TMPDIR/ph/inj.err"
set -e
if [[ -e /tmp/ooda_inject_marker_ph ]]; then
  bad "shell injection created marker file"
else
  pass "shell injection blocked (no marker)"
fi

if [[ $fail -ne 0 ]]; then
  echo "problem_hunt_smoke: FAILED" >&2
  exit 1
fi
echo "problem_hunt_smoke: PASSED"
exit 0
