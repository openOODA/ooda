#!/usr/bin/env bash
# M25 Cap vs FFI residual honesty — process-local caps do NOT seal C FFI
# job: grep residual doc marker; forbid false "FFI fully sealed/enforced shipped" claims
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
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

# Named surface: &UnsafeFFICap (path A check type In; runtime token residual)
if grep -q '&UnsafeFFICap' "$DOC"; then
  pass "doc names &UnsafeFFICap surface"
else
  bad "doc missing &UnsafeFFICap named surface"
fi
if grep -q 'CAP_FFI_PATH_A_ALPHA' "$DOC"; then
  pass "doc marker CAP_FFI_PATH_A_ALPHA"
else
  bad "doc missing CAP_FFI_PATH_A_ALPHA"
fi
# Residual: must not claim full FFI sandbox / OS dlopen isolation as shipped
if grep -qiE 'FFI fully (sealed|enforced) shipped|OS dlopen isolation (shipped|enforced)|full C TCB seal shipped' "$DOC"; then
  bad "doc claims full FFI sandbox / OS dlopen isolation shipped"
else
  pass "doc does not claim full FFI sandbox / OS dlopen isolation shipped"
fi
# Path A runtime token must be named as process-local (not denied as absent)
if grep -qE 'oo_cap_grant_ffi|process-local.*ffi|Process-local FFI token' "$DOC"; then
  pass "doc names process-local FFI token path A"
else
  bad "doc missing process-local FFI token path A wording"
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
# Path A: check-side UnsafeFFICap is In; runtime grant must stay residual
if ! grep -q 'UnsafeFFICap' oodac/check_cap_util.oo; then
  echo "FAIL path A missing UnsafeFFICap in check_cap_util" >&2; fail=1
else
  echo "OK check path A names UnsafeFFICap"
fi
# product truth: process-local grant/require for FFI is In (M156) in chs_rt_ffi.c
if grep -q 'oo_cap_grant_ffi' runtime/chs_rt_ffi.c 2>/dev/null || grep -rq 'oo_cap_grant_ffi' runtime/ 2>/dev/null; then
  echo "OK runtime has process-local oo_cap_grant_ffi (path A)"
else
  echo "FAIL runtime missing oo_cap_grant_ffi" >&2; fail=1
fi
# must not claim real OS dlopen sandbox in runtime comments as shipped full seal
if grep -rn --include='*.c' -E 'OS dlopen isolation shipped|full C TCB sealed' runtime/ 2>/dev/null | head -1 | grep -q .; then
  echo "FAIL runtime overclaims OS dlopen isolation" >&2; fail=1
else
  echo "OK runtime does not overclaim OS dlopen isolation"
fi
if ! grep -q 'cap_ffi_residual_smoke.sh' scripts/ci_product.sh; then
  echo "FAIL ci_product missing cap_ffi_residual_smoke" >&2; fail=1
else
  echo "OK ci_product wires cap_ffi residual"
fi
if ! grep -q 'cap_ffi_product_floor_smoke.sh' scripts/ci_product.sh && ! grep -q 'cap_ffi_product_floor_smoke' scripts/caps_product_floor_smoke.sh; then
  echo "FAIL product floor smoke not wired" >&2; fail=1
else
  echo "OK product floor smoke wired"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_ffi_residual_smoke: FAILED" >&2
  exit 1
fi
echo "cap_ffi_residual_smoke: PASSED"
exit 0
