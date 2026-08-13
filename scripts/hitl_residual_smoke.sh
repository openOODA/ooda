#!/usr/bin/env bash
# M24 HITL residual honesty — no interactive harness / not agent pause-resume
# job: grep residual doc marker; forbid false "HITL shipped/enforced" claims
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOC="$ROOT/bootstrap/HITL.oot"
FIX="$ROOT/fixtures/hitl_marker.oo"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

if grep -q 'HITL_RESIDUAL_ALPHA' "$DOC"; then
  pass "doc marker HITL_RESIDUAL_ALPHA"
else
  bad "doc missing HITL_RESIDUAL_ALPHA"
fi

# Residual doc must not claim HITL shipped/enforced as product truth
# (affirmative verbs only — denial list may quote the forbidden phrases)
if grep -nE '(ships|implements|enforces|provides) (interactive )?HITL|HITL (harness|testing) is (shipped|enforced|product.?green)' "$DOC"; then
  bad "HITL.md claims HITL shipped/enforced"
else
  pass "doc does not claim HITL shipped/enforced"
fi

# Must still name residual denial of interactive harness (honesty)
if grep -qE 'No.*interactive HITL|no interactive HITL|not.*interactive harness' "$DOC"; then
  pass "doc denies interactive HITL harness explicitly"
else
  bad "doc missing explicit interactive HITL harness denial"
fi
if grep -qE 'Not.*agent pause|not agent pause|no.*agent pause/resume' "$DOC"; then
  pass "doc denies agent pause/resume product"
else
  bad "doc missing agent pause/resume denial"
fi

# Must not claim harness is product-green (affirmative "is shipped" only)
if grep -qiE 'HITL harness is (shipped|enforced|green)|verify_human is (shipped|enforced)' "$DOC"; then
  bad "doc claims interactive HITL / verify_human shipped"
else
  pass "doc does not claim interactive HITL / verify_human shipped"
fi

# Fixture carries product marker form (documentation rail)
if [[ -f "$FIX" ]] && grep -qE '// HITL:[[:space:]]*pause' "$FIX"; then
  pass "fixture hitl_marker.oo has // HITL: pause"
else
  bad "fixture missing or without // HITL: pause"
fi

# Residual honesty: residual docs must not claim HITL is product-green
# Scope: bootstrap residual surface only (not DESIGN aspirational)
_hits="$(grep -rn --include='*.oot' --exclude='HITL.md' --exclude-dir=dist \
  -E 'HITL (shipped|enforced)|human-in-the-loop (shipped|enforced|green)' \
  "$ROOT/bootstrap" 2>/dev/null || true)"
if [[ -n "$_hits" ]] && echo "$_hits" | grep -vE 'not |residual|not-started|no false' | grep -q .; then
  bad "bootstrap residual claims HITL shipped elsewhere"
  echo "$_hits" >&2
else
  pass "no false HITL shipped claims in bootstrap residual surface"
fi

# Explicit: residual is fail-closed honesty, not silent green product feature
if grep -qE 'Fail-closed residual|fail-closed residual' "$DOC"; then
  pass "doc states fail-closed residual"
else
  bad "doc missing fail-closed residual wording"
fi

if [[ $fail -ne 0 ]]; then
  echo "hitl_residual_smoke: FAILED" >&2
  exit 1
fi
echo "hitl_residual_smoke: PASSED"
exit 0
