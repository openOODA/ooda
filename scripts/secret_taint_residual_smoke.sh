#!/usr/bin/env bash
# M22/M52 Static taint residual honesty — interproc/NetCap residual; path A println In
# job: grep residual doc marker; forbid false "taint tracking shipped/enforced" claims
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOC="$ROOT/bootstrap/SECRET_TAINT.md"
FIX="$ROOT/fixtures/secret_marker.oo"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

if grep -q 'SECRET_TAINT_RESIDUAL_ALPHA' "$DOC"; then
  pass "doc marker SECRET_TAINT_RESIDUAL_ALPHA"
else
  bad "doc missing SECRET_TAINT_RESIDUAL_ALPHA"
fi

# Residual doc must not claim taint tracking shipped/enforced as product truth
if grep -nE '(ships|implements|enforces|provides) (static )?taint|taint (analysis|tracking) is (shipped|enforced|product.?green)' "$DOC"; then
  bad "SECRET_TAINT.md claims taint tracking shipped/enforced"
else
  pass "doc does not claim taint tracking shipped/enforced"
fi



# NetCap / non-println sinks still residual (not blanket "no sink refuse")
if grep -qE 'NetCap|non-println' "$DOC"; then
  pass "doc names NetCap/non-println residual"
else
  bad "doc missing NetCap/non-println residual"
fi

# Path A println refuse is In — doc must state it
if grep -qE 'println.*refus|bare.IDENT|path A' "$DOC"; then
  pass "doc states path A println refuse In"
else
  bad "doc missing path A println refuse In"
fi

# Must not claim analysis is product-green
if grep -qiE 'full static taint (shipped|enforced|green)|taint analysis (shipped|enforced)' "$DOC"; then
  bad "doc claims full static taint analysis shipped"
else
  pass "doc does not claim full static taint analysis shipped"
fi

# Fixture carries product marker form
if [[ -f "$FIX" ]] && grep -qE '// SECRET:[[:space:]]*[[:alnum:]_]+' "$FIX"; then
  pass "fixture secret_marker.oo has // SECRET: name"
else
  bad "fixture missing or without // SECRET: name"
fi

_hits="$(grep -rn --include='*.md' --exclude='SECRET_TAINT.md' --exclude-dir=dist \
  -E 'taint tracking (shipped|enforced)|static taint (shipped|enforced|green)' \
  "$ROOT/bootstrap" 2>/dev/null || true)"
if [[ -n "$_hits" ]] && echo "$_hits" | grep -vE 'not |residual|not-started|no false' | grep -q .; then
  bad "bootstrap residual claims taint tracking shipped elsewhere"
  echo "$_hits" >&2
else
  pass "no false taint tracking shipped claims in bootstrap residual surface"
fi

if grep -qE 'Fail-closed residual|fail-closed residual' "$DOC"; then
  pass "doc states fail-closed residual"
else
  bad "doc missing fail-closed residual wording"
fi




if [[ $fail -ne 0 ]]; then
  echo "secret_taint_residual_smoke: FAILED" >&2
  exit 1
fi
echo "secret_taint_residual_smoke: PASSED"
exit 0
