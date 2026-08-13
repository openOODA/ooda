#!/usr/bin/env bash
# job: pure verify rails — assert_eq/assert_ne/assert pass+fail; no Python critical path
# in:  bin/ooda or oodac product path; fixtures/verify_{pass,fail}.oo
# out: exit 0 if pure verify behaves as expected
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: $OODA" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# Honesty: verify path must wire pure generator (not python3 harness).
if ! grep -q 'ooda_verify_pure.sh' "$ROOT/scripts/ooda_test_verify.sh"; then
  bad "ooda_test_verify.sh missing ooda_verify_pure.sh"
else
  pass "ooda_test_verify wires ooda_verify_pure.sh"
fi
# Critical path = non-comment lines that exec python / pure_build script.
if grep -vE '^[[:space:]]*#' "$ROOT/scripts/ooda_test_verify.sh" \
  | grep -nE 'python3|[[:space:]]ooda_test_harness\.py|oodac_pure_build\.sh' 2>/dev/null; then
  bad "ooda_test_verify.sh still invokes Python or pure_build"
else
  pass "no Python/pure_build invoke on ooda_test_verify critical path"
fi

# Direct pure generator smoke
H="$TMPDIR/verify_pure_gen.oo"
export OODA_TEST_SRC="$ROOT/fixtures/verify_pass.oo"
export OODA_TEST_HARNESS="$H"
set +e
bash "$ROOT/scripts/ooda_verify_pure.sh" >"$TMPDIR/vgen.out" 2>"$TMPDIR/vgen.err"
gr=$?
set -e
if [[ $gr -ne 0 || ! -s "$H" ]]; then
  bad "ooda_verify_pure gen verify_pass rc=$gr"
  head -15 "$TMPDIR/vgen.err" 2>/dev/null || true
elif ! grep -q 'ooda_verify_pure' "$H"; then
  bad "harness missing pure banner"
elif ! grep -q 'OK verify' "$H"; then
  bad "harness missing OK verify println"
else
  pass "ooda_verify_pure gen (verify_pass)"
fi
# Must not invoke python3 on gen path (generator is bash/awk only)
if command -v strace >/dev/null 2>&1; then
  set +e
  strace -f -e execve -o "$TMPDIR/vgen.strace" \
    bash "$ROOT/scripts/ooda_verify_pure.sh" >/dev/null 2>&1
  set -e
  if grep -E 'python3?|"python' "$TMPDIR/vgen.strace" 2>/dev/null; then
    bad "ooda_verify_pure exec'ed python"
  else
    pass "ooda_verify_pure no python exec"
  fi
fi

# Product: pass fixture
set +e
"$OODA" test "$ROOT/fixtures/verify_pass.oo" >"$TMPDIR/vp.out" 2>"$TMPDIR/vp.err"
tp=$?
set -e
if [[ $tp -ne 0 ]] || ! grep -q "OK verify" "$TMPDIR/vp.out"; then
  bad "verify_pass exit=$tp"
  head -20 "$TMPDIR/vp.err" "$TMPDIR/vp.out" 2>/dev/null || true
else
  pass "product test verify_pass"
fi

# Product: fail fixture
set +e
"$OODA" test "$ROOT/fixtures/verify_fail.oo" >"$TMPDIR/vf.out" 2>"$TMPDIR/vf.err"
tf=$?
set -e
if [[ $tf -eq 0 ]]; then
  bad "verify_fail accepted (exit 0)"
else
  pass "product test fail-closed verify_fail (rc=$tf)"
fi

# assert_ne! + assert!
cat >"$TMPDIR/vne.oo" <<'EOF'
pub fn add(a: Int, b: Int) -> Int { return a + b; }
verify add {
    assert_ne!(add(1, 1), 3);
    assert!(add(2, 2) == 4);
}
pub fn main() { println(1); }
EOF
set +e
"$OODA" test "$TMPDIR/vne.oo" >"$TMPDIR/vne.out" 2>"$TMPDIR/vne.err"
vt=$?
set -e
if [[ $vt -ne 0 ]] || ! grep -q 'OK verify' "$TMPDIR/vne.out"; then
  bad "assert_ne/assert! verify (exit=$vt)"
  head -15 "$TMPDIR/vne.out" "$TMPDIR/vne.err" 2>/dev/null || true
else
  pass "assert_ne + assert! verify"
fi

# check-only (no verify blocks)
cat >"$TMPDIR/vchk.oo" <<'EOF'
pub fn main() { println(1); }
EOF
set +e
"$OODA" test "$TMPDIR/vchk.oo" >"$TMPDIR/vchk.out" 2>"$TMPDIR/vchk.err"
tc=$?
set -e
if [[ $tc -ne 0 ]]; then
  bad "check-only no-verify exit=$tc"
else
  pass "check-only (no verify blocks)"
fi

# Line lock
for f in ooda_verify_pure.sh ooda_test_verify.sh verify_pure_smoke.sh; do
  n=$(wc -l <"$ROOT/scripts/$f")
  if [[ "$n" -gt 256 ]]; then
    bad "$f over MAX_LINES 256 ($n)"
  else
    pass "$f lines=$n (<=256)"
  fi
done

if [[ $fail -ne 0 ]]; then
  echo "verify_pure_smoke: FAILED" >&2
  exit 1
fi
echo "verify_pure_smoke: PASSED"
exit 0
