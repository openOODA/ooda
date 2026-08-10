#!/usr/bin/env bash
# M25 Cap vs FFI residual honesty — process-local caps do NOT seal C FFI
# job: grep residual doc marker; forbid false "FFI fully sealed/enforced shipped" claims
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOC="$ROOT/bootstrap/CAP_FFI.md"
FIX="$ROOT/fixtures/ffi_marker.oo"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

if grep -q 'CAP_FFI_RESIDUAL_ALPHA' "$DOC"; then
  pass "doc marker CAP_FFI_RESIDUAL_ALPHA"
else
  bad "doc missing CAP_FFI_RESIDUAL_ALPHA"
fi

# Residual doc must not claim FFI fully sealed/enforced as product truth
# (affirmative verbs only — denial list may quote the forbidden phrases)
if grep -nE '(ships|implements|enforces|provides) (full )?(FFI (sandbox|seal)|sealed FFI)|FFI (sandbox|boundary) is (shipped|enforced|product.?green)|FFI fully (sealed|enforced) shipped' "$DOC"; then
  bad "CAP_FFI.md claims FFI fully sealed/enforced shipped"
else
  pass "doc does not claim FFI fully sealed/enforced shipped"
fi

# Must still name residual: process-local caps do NOT seal C FFI / dlopen / raw pointers
if grep -qE 'do \*\*NOT\*\* seal|do NOT seal|do not seal' "$DOC" && grep -qE 'C FFI|dlopen|raw pointer' "$DOC"; then
  pass "doc denies process-local seal over C FFI / dlopen / raw pointers"
else
  bad "doc missing explicit process-local caps do-not-seal C FFI denial"
fi

# Must name DESIGN tension residual
if grep -qE 'DESIGN tension|tension residual' "$DOC"; then
  pass "doc names DESIGN tension residual"
else
  bad "doc missing DESIGN tension residual wording"
fi

# Named surface only: &UnsafeFFICap (doc name; type not required)
if grep -q '&UnsafeFFICap' "$DOC"; then
  pass "doc names &UnsafeFFICap surface"
else
  bad "doc missing &UnsafeFFICap named surface"
fi
if grep -qiE 'UnsafeFFICap (type|grant|token) (shipped|enforced|implemented)' "$DOC"; then
  bad "doc claims &UnsafeFFICap type/grant shipped"
else
  pass "doc does not claim &UnsafeFFICap type shipped"
fi

# Fixture carries product marker form (documentation rail)
if [[ -f "$FIX" ]] && grep -qE '// FFI:[[:space:]]*residual' "$FIX"; then
  pass "fixture ffi_marker.oo has // FFI: residual"
else
  bad "fixture missing or without // FFI: residual"
fi

# Residual honesty: residual docs must not claim FFI fully sealed elsewhere
# Scope: bootstrap residual surface only (not DESIGN aspirational)
_hits="$(grep -rn --include='*.md' --exclude='CAP_FFI.md' --exclude-dir=dist \
  -E 'FFI fully (sealed|enforced)|FFI sandbox (shipped|enforced|green)' \
  "$ROOT/bootstrap" 2>/dev/null || true)"
if [[ -n "$_hits" ]] && echo "$_hits" | grep -vE 'not |residual|not-started|no false|do NOT' | grep -q .; then
  bad "bootstrap residual claims FFI fully sealed elsewhere"
  echo "$_hits" >&2
else
  pass "no false FFI fully sealed claims in bootstrap residual surface"
fi

# Explicit: residual is fail-closed honesty, not silent green product feature
if grep -qE 'Fail-closed residual|fail-closed residual' "$DOC"; then
  pass "doc states fail-closed residual"
else
  bad "doc missing fail-closed residual wording"
fi


# M25 strengthen: dual-claim ceiling + product truth + ci wire
if ! grep -qE 'CAP_FFI|do not.*seal C FFI|C FFI' bootstrap/STATIC_CAPS.md; then
  echo "FAIL STATIC_CAPS missing Cap/FFI residual link" >&2; fail=1
else
  echo "OK STATIC_CAPS names Cap/FFI residual ceiling"
fi
if ! grep -qE 'CAP_FFI|do not.*seal C|interop' bootstrap/CAPS_MATRIX.md; then
  echo "FAIL CAPS_MATRIX missing Cap/FFI residual note" >&2; fail=1
else
  echo "OK CAPS_MATRIX names Cap/FFI residual ceiling"
fi
if grep -rn --include='*.oo' -E 'UnsafeFFICap' oodac/ 2>/dev/null | grep -v residual | head -1 | grep -q .; then
  echo "WARN UnsafeFFICap appears in oodac sources (check not shipped as green)"
fi
# product truth: no grant/require for UnsafeFFICap in runtime
if grep -rn --include='*.c' --include='*.h' -E 'oo_cap_grant_unsafe|UnsafeFFICap' runtime/ 2>/dev/null | head -1 | grep -q .; then
  echo "FAIL runtime appears to implement UnsafeFFICap" >&2; fail=1
else
  echo "OK runtime has no UnsafeFFICap grant/token"
fi
if ! grep -q 'cap_ffi_residual_smoke.sh' scripts/ci_product.sh; then
  echo "FAIL ci_product missing cap_ffi_residual_smoke" >&2; fail=1
else
  echo "OK ci_product wires cap_ffi residual"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_ffi_residual_smoke: FAILED" >&2
  exit 1
fi
echo "cap_ffi_residual_smoke: PASSED"
exit 0
