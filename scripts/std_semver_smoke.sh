#!/usr/bin/env bash
# job: pure std/semver.oo check + multi-build fixture
# in:  oodac, std/semver.oo, fixtures/std_semver_main.oo
# out: exit 0 if core major.minor.patch parse is real on pure path
# residual: prerelease/build NOT accepted (invalid); no SemVer struct
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo ERR_NO_OODAC >&2; exit 1; }

set +e
"$OODAC" check "$ROOT/std/semver.oo" \
  >"$TMPDIR/std_semver_ck.out" 2>"$TMPDIR/std_semver_ck.err"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  bad "check std/semver.oo"
  head -12 "$TMPDIR/std_semver_ck.err" 2>/dev/null || true
else
  pass "check std/semver.oo"
fi

# Residual honesty: no false full-semver / DESIGN-done claims
if grep -qiE 'full semver DESIGN|prerelease fully|build metadata fully' \
  "$ROOT/std/semver.oo"; then
  bad "honesty std/semver.oo forbidden full-semver claim"
else
  if grep -qiE 'prerelease|build' "$ROOT/std/semver.oo" \
    && grep -qiE 'residual|NOT accepted|not accepted' "$ROOT/std/semver.oo"; then
    pass "honesty residual std/semver.oo"
  else
    bad "honesty residual missing std/semver.oo"
  fi
fi

f="$ROOT/fixtures/std_semver_main.oo"
set +e
OODAC_BIN="$OODAC" "$OODAC" build "$f" "$TMPDIR/std_semver_main" \
  >"$TMPDIR/std_semver_b.out" 2>"$TMPDIR/std_semver_b.err"
rc=$?
set -e
if [[ $rc -ne 0 || ! -x "$TMPDIR/std_semver_main" ]]; then
  bad "build fixtures/std_semver_main.oo"
  head -16 "$TMPDIR/std_semver_b.err" 2>/dev/null || true
  head -16 "$TMPDIR/std_semver_b.out" 2>/dev/null || true
else
  set +e
  "$TMPDIR/std_semver_main" >"$TMPDIR/std_semver_r.out" 2>"$TMPDIR/std_semver_r.err"
  rr=$?
  set -e
  out="$(cat "$TMPDIR/std_semver_r.out" 2>/dev/null || true)"
  err="$(cat "$TMPDIR/std_semver_r.err" 2>/dev/null || true)"
  if [[ $rr -ne 0 ]]; then
    bad "run fixtures/std_semver_main.oo (rc=$rr err=$err)"
  elif ! echo "$out" | grep -q 'semver-ok'; then
    bad "run missing semver-ok (got: $out)"
  else
    pass "build+run fixtures/std_semver_main.oo"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "std_semver_smoke: FAILED" >&2
  exit 1
fi
echo "std_semver_smoke: PASSED"
exit 0
