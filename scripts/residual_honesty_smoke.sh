#!/usr/bin/env bash
# job: mechanical residual honesty gate for M3/M2 residual surface
# in:  product residual docs + ci_product rails + honesty probes
# out: exit 0 only if no forbidden half-truth banners remain
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

fail=0
hits="$(mktemp)"
trap 'rm -f "$hits"' EXIT

# Forbidden exact-ish claims on the LIVE residual surface.
# Scope excludes RELEASE_NOTES_*, dist/, and this smoke's own pattern list.
# Not forbidden: future-goal wording in FUZZ_DEFER ("without Python residual").
scan() {
  local pat="$1"
  grep -rn --include='*.md' --include='*.sh' \
    --exclude='RELEASE_NOTES*' \
    --exclude='residual_honesty_smoke.sh' \
    --exclude-dir=dist --exclude-dir=.git \
    -e "$pat" \
    README.md bootstrap scripts qa 2>/dev/null || true
}

# High-signal false claims (substring match; keep precise)
{
  scan 'Python residual;'
  scan 'Python residual)'
  scan 'Python residual,'
  scan 'Python residual —'
  scan 'Python residual -'
  scan 'Python residual (`'
  scan 'harness is Python residual'
  scan 'harness still Python'
  scan 'un-gated (Python residual'
  scan 'Python harness residual'
  scan 'strip until M2'
  scan 'default **`PURE_NO_ARC=1`**'
  scan 'Default **`PURE_NO_ARC=1`**'
  scan 'PURE_NO_ARC=1`** default'
  scan 'PURE_NO_ARC=1` default'
  scan 'under **`PURE_NO_ARC=1` strip'
} >"$hits" || true

# Drop allowlisted future-goal line in FUZZ_DEFER
if [[ -s "$hits" ]]; then
  grep -v 'Broader types / multi-param without Python residual' "$hits" \
    | grep -v 'not Python residual' \
    | grep -v 'no Python residual' \
    | grep -v 'without Python residual' \
    >"${hits}.f" || true
  mv "${hits}.f" "$hits"
fi

if [[ -s "$hits" ]]; then
  echo "FAIL residual_honesty: stale half-truths still present:" >&2
  sort -u "$hits" >&2
  fail=1
else
  echo "OK residual_honesty: no stale Python/--fuzz or strip-default claims"
fi

if ! grep -q 'ooda_fuzz_pure.sh' scripts/ooda_test_verify.sh; then
  echo "FAIL residual_honesty: ooda_test_verify.sh missing ooda_fuzz_pure.sh" >&2
  fail=1
else
  echo "OK residual_honesty: ooda_test_verify wires ooda_fuzz_pure.sh"
fi

if ! grep -q 'ooda_verify_pure.sh' scripts/ooda_test_verify.sh; then
  echo "FAIL residual_honesty: ooda_test_verify.sh missing ooda_verify_pure.sh" >&2
  fail=1
else
  echo "OK residual_honesty: ooda_test_verify wires ooda_verify_pure.sh"
fi

# M50: no Python harness / multi pure-build on ooda test critical path (ignore comments)
if grep -vE '^[[:space:]]*#' scripts/ooda_test_verify.sh \
  | grep -nE 'python3|[[:space:]]ooda_test_harness\.py|oodac_pure_build\.sh' 2>/dev/null; then
  echo "FAIL residual_honesty: ooda_test_verify still invokes Python harness" >&2
  fail=1
else
  echo "OK residual_honesty: no Python on ooda_test_verify critical path"
fi
if grep -vE '^[[:space:]]*#' scripts/ooda_test_verify.sh \
  | grep -nE 'oodac_pure_build\.sh' 2>/dev/null; then
  echo "FAIL residual_honesty: ooda_test_verify invokes pure multi build" >&2
  fail=1
else
  echo "OK residual_honesty: ooda_test_verify has no pure multi build"
fi

if grep -q 'PURE_NO_ARC="${PURE_NO_ARC:-0}"' scripts/bootstrap_no_cargo.sh; then
  echo "OK residual_honesty: bootstrap default PURE_NO_ARC=0"
else
  echo "FAIL residual_honesty: bootstrap_no_cargo default is not PURE_NO_ARC=0" >&2
  fail=1
fi

# p3 / product rails must not print Python residual pass banners
if grep -nE 'pass ".*Python residual' scripts/p3_no_cargo_smoke.sh scripts/product_pure_dispatch_smoke.sh 2>/dev/null; then
  echo "FAIL residual_honesty: ci rail still pass-banners Python residual" >&2
  fail=1
else
  echo "OK residual_honesty: p3/product smokes have no Python residual pass banners"
fi

if [[ $fail -ne 0 ]]; then
  echo "residual_honesty_smoke: FAILED" >&2
  exit 1
fi
echo "residual_honesty_smoke: PASSED"
exit 0
