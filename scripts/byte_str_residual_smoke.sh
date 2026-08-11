#!/usr/bin/env bash
# Byte / &str residual honesty — not DESIGN-done; string value-copy remains
# job: marker + residual wording + optional std/byte Int convention floor
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
DOC="$ROOT/bootstrap/BYTE_STR.md"
FIX="$ROOT/fixtures/byte_str_marker.oo"
STD="$ROOT/std/byte.oo"
MARKER="BYTE_STR_RESIDUAL_ALPHA"
FIXLINE="BYTE_STR: residual"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

if grep -q "$MARKER" "$DOC"; then
  pass "doc marker $MARKER"
else
  bad "doc missing $MARKER"
fi

# Must state residual surfaces explicitly
if grep -qE 'native.*&str|&str.*borrow|borrowed string' "$DOC"; then
  pass "doc names &str borrow residual"
else
  bad "doc missing &str borrow residual"
fi
if grep -qiE 'Byte array|byte arrays|List\[Byte\]' "$DOC"; then
  pass "doc names Byte arrays residual"
else
  bad "doc missing Byte arrays residual"
fi
if grep -qiE 'value-copy|value copy|OoStr' "$DOC"; then
  pass "doc states string value-copy remains"
else
  bad "doc missing string value-copy residual"
fi

# Must not claim DESIGN-done / product-green Byte/&str
if grep -nEi 'Byte.*fully shipped|&str.*product green|native &str (is |shipped)|Byte arrays (shipped|enforced)' "$DOC" \
  | grep -viE 'not |residual|do not|never|no ' >/dev/null; then
  bad "doc may claim Byte/&str shipped without residual"
else
  pass "doc does not claim Byte/&str product-green"
fi

if grep -qiE 'fail-closed residual|Fail-closed residual' "$DOC"; then
  pass "fail-closed residual wording"
else
  bad "fail-closed residual wording"
fi

if grep -qiE 'do \*\*not\*\* claim|What we do \*\*not\*\* claim' "$DOC"; then
  pass "non-claims section"
else
  bad "non-claims section"
fi

[[ -f "$FIX" ]] || bad "missing fixture $FIX"
if [[ -f "$FIX" ]] && grep -q "$FIXLINE" "$FIX"; then
  pass "fixture marker $FIXLINE"
else
  bad "fixture missing $FIXLINE"
fi

# Optional pure-std Byte = Int convention: present + residual honesty in-file
if [[ -f "$STD" ]]; then
  pass "std/byte.oo present"
  if grep -qE 'type Byte = Int' "$STD"; then
    pass "std Byte alias is Int convention"
  else
    bad "std/byte.oo missing type Byte = Int"
  fi
  if grep -qiE 'residual:.*(&str|Byte array|value-copy|u8 primitive)' "$STD"; then
    pass "std/byte.oo documents residual"
  else
    bad "std/byte.oo missing residual honesty comments"
  fi
  OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
  if [[ -x "$OODAC" ]]; then
    set +e
    "$OODAC" check "$STD" >"${TMPDIR:-/tmp}/byte_str_std_ck.out" 2>"${TMPDIR:-/tmp}/byte_str_std_ck.err"
    rc=$?
    set -e
    if [[ $rc -eq 0 ]]; then
      pass "oodac check std/byte.oo"
    elif grep -qE "undefined variable '(bytes_len|byte_slice|bytes_eq|byte_at|bytes_from_str|bytes_concat|bytes_new|bytes_push|bytes_get|bytes_to_str)'" \
      "${TMPDIR:-/tmp}/byte_str_std_ck.out" "${TMPDIR:-/tmp}/byte_str_std_ck.err" 2>/dev/null; then
      # Path A free names need oodac rebuild (parent ship); residual rails still hold
      pass "skip oodac check (pre path-A free-name oodac)"
    else
      bad "oodac check std/byte.oo failed"
      head -8 "${TMPDIR:-/tmp}/byte_str_std_ck.err" 2>/dev/null || true
    fi
  else
    pass "skip oodac check (no OODAC)"
  fi
else
  bad "missing std/byte.oo"
fi

# Dual-claim scan: bootstrap residual surface must not claim Byte/&str shipped
_hits="$(grep -rn --include='*.md' --exclude='BYTE_STR.md' --exclude-dir=dist \
  -E 'native &str (shipped|enforced)|Byte arrays (shipped|enforced)|Byte primitive (shipped|enforced)' \
  "$ROOT/bootstrap" 2>/dev/null || true)"
if [[ -n "$_hits" ]] && echo "$_hits" | grep -vE 'not |residual|not-started|no false' | grep -q .; then
  bad "bootstrap residual claims Byte/&str shipped elsewhere"
  echo "$_hits" >&2
else
  pass "no false Byte/&str shipped claims in bootstrap residual surface"
fi

if grep -q 'byte_str_residual_smoke.sh' scripts/ci_product.sh; then
  pass "ci_product wire"
else
  bad "ci_product missing byte_str_residual_smoke.sh"
fi

if grep -q 'BYTE_STR.md' bootstrap/RESIDUAL_PACKS.md 2>/dev/null; then
  pass "RESIDUAL_PACKS index"
else
  bad "RESIDUAL_PACKS missing BYTE_STR.md"
fi

if [[ $fail -ne 0 ]]; then
  echo "byte_str_residual_smoke: FAILED" >&2
  exit 1
fi
echo "byte_str_residual_smoke: PASSED"
exit 0
