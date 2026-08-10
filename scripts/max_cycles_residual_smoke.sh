#!/usr/bin/env bash
# M48 MaxCycles residual honesty — path A while fuel In; residual for/OS/attribute
# Attack: residual must NOT still claim "names only" after path A product In
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOC="$ROOT/bootstrap/MAX_CYCLES.md"
FIX="$ROOT/fixtures/max_cycles_marker.oo"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

if grep -q 'MAX_CYCLES_RESIDUAL_ALPHA' "$DOC"; then
  pass "doc marker MAX_CYCLES_RESIDUAL_ALPHA"
else
  bad "doc missing MAX_CYCLES_RESIDUAL_ALPHA"
fi

if grep -nE '(enforced by|uses|ships|via|implements) (cgroup|cpulimit|RLIMIT_CPU)' "$DOC"; then
  bad "MAX_CYCLES.md claims OS CPU isolation"
else
  pass "doc does not claim cgroup/cpulimit/RLIMIT_CPU enforcement"
fi
if grep -qE 'cgroup|RLIMIT|not OS' "$DOC"; then
  pass "doc denies OS isolation explicitly"
else
  bad "doc missing explicit OS isolation denial"
fi

if grep -qE 'path A|while fuel|__oo_mc|max_cycles_fuel_inject' "$DOC"; then
  pass "doc states path A while fuel product surface"
else
  bad "doc missing path A while fuel In wording"
fi

# Attack 1: residual must NOT claim product is names-only after enforce In
_bad_names="$(
  grep -nE 'names only|named marker only|named surface only|neither is lowered|marker only, not enforced' "$DOC" 2>/dev/null \
    | grep -viE '#\[MaxCycles\]|attribute|residual name|was M21|for loop|static WCET' || true
)"
if [[ -n "$_bad_names" ]]; then
  bad "doc still claims names-only for product after path A In"
  echo "$_bad_names" >&2
else
  pass "doc does not claim names-only product surface"
fi

if [[ -f "$FIX" ]] && grep -qE '// MAX_CYCLES:[[:space:]]*[0-9]+' "$FIX"; then
  pass "fixture max_cycles_marker.oo has // MAX_CYCLES: N"
else
  bad "fixture missing or without // MAX_CYCLES: N"
fi
if [[ -f "$FIX" ]] && grep -qiE 'Not enforced|no Backend-C loop fuel' "$FIX"; then
  bad "marker fixture still claims while fuel not enforced"
else
  pass "marker fixture does not deny path A fuel"
fi

if grep -q 'max_cycles_enforce_smoke.sh' "$ROOT/scripts/ci_product.sh" \
  || grep -q 'max_cycles_smoke.sh' "$ROOT/scripts/ci_product.sh"; then
  pass "ci_product wires path A product smoke"
else
  bad "ci_product missing max_cycles product smoke"
fi
if grep -q 'max_cycles_residual_smoke.sh' "$ROOT/scripts/ci_product.sh"; then
  pass "ci_product wires residual smoke"
else
  bad "ci_product missing residual smoke"
fi

if grep -qE 'for|\#\[MaxCycles\]|cgroup' "$DOC"; then
  pass "doc keeps residual non-claims surface"
else
  bad "doc missing residual non-claims"
fi

if [[ $fail -ne 0 ]]; then
  echo "max_cycles_residual_smoke: FAILED" >&2
  exit 1
fi
echo "max_cycles_residual_smoke: PASSED"
exit 0
