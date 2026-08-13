#!/usr/bin/env bash
# M54 MaxCycles path B — range-for fuel under // MAX_CYCLES: N
# residual: recursion / non-range for / OS cgroup / #[MaxCycles]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

PASS="$ROOT/fixtures/max_cycles_for_pass.oo"
FAIL="$ROOT/fixtures/max_cycles_for_fail.oo"
DOC="$ROOT/bootstrap/MAX_CYCLES.oot"
RT="$ROOT/runtime"
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; exit 1; }

[[ -f "$PASS" && -f "$FAIL" ]] || bad "missing for fixtures"

set +e
raw_p="$("$OODAC" emit-c "$PASS" 2>"$TMPDIR/mcf_pass.err")"
rp=$?
set -e
[[ $rp -eq 0 ]] || bad "for_pass emit rc=$rp"
echo "$raw_p" | grep -qE $'^ERR\t' && bad "for_pass ERR line" || true
echo "$raw_p" | grep -q '__oo_mc' || bad "for_pass missing fuel counter"
echo "$raw_p" | grep -qE 'for[[:space:]]*\(' || bad "for_pass missing for"
if echo "$raw_p" | grep -E 'for[[:space:]]*\([^)]*__oo_mc' >/dev/null 2>&1; then
  bad "for_pass put fuel in for-header"
fi
pass "for_pass emit has body fuel (not header)"
cp_p="$TMPDIR/mcf_pass.c"
echo "$raw_p" | grep -v '🚀\|Running main' >"$cp_p" || true
gcc -O0 -I"$RT" "$RT/chs_rt.c" "$cp_p" -o "$TMPDIR/mcf_pass.bin" -lm
out_p="$(timeout 3 "$TMPDIR/mcf_pass.bin" 2>&1 || true)"
echo "$out_p" | grep -qE '3' || bad "for_pass run expected 3 got: $out_p"
pass "for_pass run under N (got 3)"

set +e
raw_f="$("$OODAC" emit-c "$FAIL" 2>"$TMPDIR/mcf_fail.err")"
rf=$?
set -e
[[ $rf -eq 0 ]] || bad "for_fail emit rc=$rf"
echo "$raw_f" | grep -q '__oo_mc' || bad "for_fail missing fuel"
cf="$TMPDIR/mcf_fail.c"
echo "$raw_f" | grep -v '🚀\|Running main' >"$cf" || true
gcc -O0 -I"$RT" "$RT/chs_rt.c" "$cf" -o "$TMPDIR/mcf_fail.bin" -lm
set +e
out_f="$(timeout 3 "$TMPDIR/mcf_fail.bin" 2>&1)"
rcf=$?
set -e
[[ $rcf -ne 0 ]] || bad "for_fail should non-zero"
echo "$out_f" | grep -qiE 'max_cycles' || bad "for_fail missing max_cycles out=$out_f"
pass "for_fail exit non-zero + ERR max_cycles (rc=$rcf)"

[[ -f "$DOC" ]] || bad "missing MAX_CYCLES.md"
grep -qE 'path B|range-for|range.?for' "$DOC" || bad "doc missing path B for fuel"
grep -qE 'cgroup|not OS' "$DOC" || bad "doc missing OS residual"
pass "doc path B + OS residual honest"

if grep -q 'max_cycles_for_enforce_smoke' "$ROOT/scripts/ci_product.sh"; then
  pass "ci_product wires max_cycles_for_enforce_smoke"
else
  bad "ci_product missing max_cycles_for_enforce_smoke"
fi

echo "max_cycles_for_enforce_smoke: PASSED"
