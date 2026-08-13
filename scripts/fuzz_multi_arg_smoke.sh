#!/usr/bin/env bash
# job: M49 pure Int arity-2/3 multi-arg fuzz + weak arity + arity≥4 fail-closed
# in:  bin/ooda; fixtures/fuzz_int_multi*.oo / fuzz_int_multi3_*.oo / fuzz_*_multi_weak.oo
# out: exit 0 if multi-arg rails behave as expected
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: $OODA" >&2
  exit 1
fi

# --- arity-2 pass / fail (M46 regression rail) ---
set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_int_multi_add.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_multi_pass.out" 2>"$TMPDIR/fuzz_multi_pass.err"
prc=$?
set -e
if [[ $prc -eq 0 ]]; then
  pass "fuzz multi-arg Int arity-2 pass"
else
  bad "fuzz multi-arg Int arity-2 pass rc=$prc"
  head -20 "$TMPDIR/fuzz_multi_pass.err" "$TMPDIR/fuzz_multi_pass.out" 2>/dev/null || true
fi

set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_int_multi_fail.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_multi_fail.out" 2>"$TMPDIR/fuzz_multi_fail.err"
frc=$?
set -e
if [[ $frc -ne 0 ]]; then
  pass "fuzz multi-arg Int arity-2 fail-rail (rc=$frc)"
else
  bad "fuzz multi-arg Int arity-2 fail-rail expected non-zero"
  head -20 "$TMPDIR/fuzz_multi_fail.out" 2>/dev/null || true
fi

# --- arity-3 pass / fail (M49) ---
set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_int_multi3_add.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_multi3_pass.out" 2>"$TMPDIR/fuzz_multi3_pass.err"
p3=$?
set -e
if [[ $p3 -eq 0 ]]; then
  pass "fuzz multi-arg Int arity-3 pass"
else
  bad "fuzz multi-arg Int arity-3 pass rc=$p3"
  head -20 "$TMPDIR/fuzz_multi3_pass.err" "$TMPDIR/fuzz_multi3_pass.out" 2>/dev/null || true
fi

set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_int_multi3_fail.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_multi3_fail.out" 2>"$TMPDIR/fuzz_multi3_fail.err"
f3=$?
set -e
if [[ $f3 -ne 0 ]]; then
  pass "fuzz multi-arg Int arity-3 fail-rail (rc=$f3)"
else
  bad "fuzz multi-arg Int arity-3 fail-rail expected non-zero"
  head -20 "$TMPDIR/fuzz_multi3_fail.out" 2>/dev/null || true
fi

# --- arity≥4 pass ---
set +e
timeout 30 "$OODA" test "$ROOT/fixtures/fuzz_int_multi_arity4.oo" --fuzz 5 \
  >"$TMPDIR/fuzz_multi_a4.out" 2>"$TMPDIR/fuzz_multi_a4.err"
arc=$?
set -e
if [[ $arc -eq 0 ]]; then
  pass "fuzz multi-arg arity>=4 pass"
else
  bad "fuzz multi-arg arity>=4 pass (rc=$arc)"
  head -20 "$TMPDIR/fuzz_multi_a4.err" "$TMPDIR/fuzz_multi_a4.out" 2>/dev/null || true
fi

# --- weak arity: non-Int multi-arg params fail-closed ---
set +e
timeout 30 "$OODA" test "$ROOT/fixtures/fuzz_int_multi_weak.oo" --fuzz 5 \
  >"$TMPDIR/fuzz_multi_weak.out" 2>"$TMPDIR/fuzz_multi_weak.err"
wrc=$?
set -e
if [[ $wrc -ne 0 ]] && grep -qiE 'all Int params|multi-arg|fail-closed' \
  "$TMPDIR/fuzz_multi_weak.out" "$TMPDIR/fuzz_multi_weak.err" 2>/dev/null; then
  pass "fuzz multi-arg weak non-Int params fail-closed (rc=$wrc)"
else
  bad "fuzz multi-arg weak non-Int must fail-closed with msg (rc=$wrc)"
  head -20 "$TMPDIR/fuzz_multi_weak.err" "$TMPDIR/fuzz_multi_weak.out" 2>/dev/null || true
fi

# --- M56 Bool arity-2 pass / fail ---
set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_bool_multi_and.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_bool_multi_pass.out" 2>"$TMPDIR/fuzz_bool_multi_pass.err"
bmp=$?
set -e
if [[ $bmp -eq 0 ]]; then
  pass "fuzz multi-arg Bool arity-2 pass"
else
  bad "fuzz multi-arg Bool arity-2 pass rc=$bmp"
  head -20 "$TMPDIR/fuzz_bool_multi_pass.err" "$TMPDIR/fuzz_bool_multi_pass.out" 2>/dev/null || true
fi

set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_bool_multi_fail.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_bool_multi_fail.out" 2>"$TMPDIR/fuzz_bool_multi_fail.err"
bmf=$?
set -e
if [[ $bmf -ne 0 ]]; then
  pass "fuzz multi-arg Bool arity-2 fail-rail (rc=$bmf)"
else
  bad "fuzz multi-arg Bool arity-2 fail-rail expected non-zero"
  head -20 "$TMPDIR/fuzz_bool_multi_fail.out" 2>/dev/null || true
fi

# --- weak: Bool multi arity≥3 pass ---
set +e
timeout 30 "$OODA" test "$ROOT/fixtures/fuzz_bool_multi_weak.oo" --fuzz 5 \
  >"$TMPDIR/fuzz_bool_weak.out" 2>"$TMPDIR/fuzz_bool_weak.err"
brc=$?
set -e
if [[ $brc -eq 0 ]]; then
  pass "fuzz multi-arg bool arity>=3 pass"
else
  bad "fuzz multi-arg bool arity>=3 pass (rc=$brc)"
  head -20 "$TMPDIR/fuzz_bool_weak.err" "$TMPDIR/fuzz_bool_weak.out" 2>/dev/null || true
fi

# --- M106 String arity-2 pass / fail ---
set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_string_multi_id.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_str_multi_pass.out" 2>"$TMPDIR/fuzz_str_multi_pass.err"
smp=$?
set -e
if [[ $smp -eq 0 ]]; then
  pass "fuzz multi-arg String arity-2 pass"
else
  bad "fuzz multi-arg String arity-2 pass rc=$smp"
  head -20 "$TMPDIR/fuzz_str_multi_pass.err" "$TMPDIR/fuzz_str_multi_pass.out" 2>/dev/null || true
fi

set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_string_multi_fail.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_str_multi_fail.out" 2>"$TMPDIR/fuzz_str_multi_fail.err"
smf=$?
set -e
if [[ $smf -ne 0 ]]; then
  pass "fuzz multi-arg String arity-2 fail-rail (rc=$smf)"
else
  bad "fuzz multi-arg String arity-2 fail-rail expected non-zero"
  head -20 "$TMPDIR/fuzz_str_multi_fail.out" 2>/dev/null || true
fi

# --- weak: String multi arity≥3 pass ---
set +e
timeout 30 "$OODA" test "$ROOT/fixtures/fuzz_string_multi_weak.oo" --fuzz 5 \
  >"$TMPDIR/fuzz_str_weak.out" 2>"$TMPDIR/fuzz_str_weak.err"
srcw=$?
set -e
if [[ $srcw -eq 0 ]]; then
  pass "fuzz multi-arg string arity>=3 pass"
else
  bad "fuzz multi-arg string arity>=3 pass (rc=$srcw)"
  head -20 "$TMPDIR/fuzz_str_weak.err" "$TMPDIR/fuzz_str_weak.out" 2>/dev/null || true
fi

# --- M137 List arity-2 pass / fail ---
set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_list_multi_id.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_list_multi_pass.out" 2>"$TMPDIR/fuzz_list_multi_pass.err"
lmp=$?
set -e
if [[ $lmp -eq 0 ]]; then
  pass "fuzz multi-arg List arity-2 pass"
else
  bad "fuzz multi-arg List arity-2 pass rc=$lmp"
  head -20 "$TMPDIR/fuzz_list_multi_pass.err" "$TMPDIR/fuzz_list_multi_pass.out" 2>/dev/null || true
fi

set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_list_multi_fail.oo" --fuzz 20 \
  >"$TMPDIR/fuzz_list_multi_fail.out" 2>"$TMPDIR/fuzz_list_multi_fail.err"
lmf=$?
set -e
if [[ $lmf -ne 0 ]]; then
  pass "fuzz multi-arg List arity-2 fail-rail (rc=$lmf)"
else
  bad "fuzz multi-arg List arity-2 fail-rail expected non-zero"
  head -20 "$TMPDIR/fuzz_list_multi_fail.out" 2>/dev/null || true
fi

# --- residual: multi-arg List domain pass ---
set +e
timeout 30 "$OODA" test "$ROOT/fixtures/fuzz_list_multi_weak.oo" --fuzz 5 \
  >"$TMPDIR/fuzz_list_weak.out" 2>"$TMPDIR/fuzz_list_weak.err"
lmw=$?
set -e
if [[ $lmw -eq 0 ]]; then
  pass "fuzz multi-arg list domain pass"
else
  bad "fuzz multi-arg list domain pass (rc=$lmw)"
  head -20 "$TMPDIR/fuzz_list_weak.err" "$TMPDIR/fuzz_list_weak.out" 2>/dev/null || true
fi

# single-arg depth still works (unchanged domain)
set +e
timeout 60 "$OODA" test "$ROOT/fixtures/fuzz_int_add.oo" --fuzz 10 \
  >"$TMPDIR/fuzz_multi_single.out" 2>"$TMPDIR/fuzz_multi_single.err"
src=$?
set -e
if [[ $src -eq 0 ]]; then
  pass "fuzz single-arg Int still pass"
else
  bad "fuzz single-arg Int regression rc=$src"
fi

if [[ $fail -ne 0 ]]; then
  echo "fuzz_multi_arg_smoke: FAILED" >&2
  exit 1
fi
echo "fuzz_multi_arg_smoke: PASSED"
exit 0
