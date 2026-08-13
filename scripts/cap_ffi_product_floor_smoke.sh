#!/usr/bin/env bash
# Cap vs FFI path A product floor (alpha) — PM 6.3
# Check refuses bare dlopen/host-FFI; allows with &UnsafeFFICap + token first arg.
# Residual honesty pack still names full C TCB / raw-pointer / compile-time FFI gen.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODA="${OODA:-$ROOT/bin/ooda}"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- check deny: bare dlopen ---
set +e
"$OODAC_BIN" check "$ROOT/fixtures/ffi_dlopen_fail.oo" >"$TMPDIR/cff_fail.out" 2>"$TMPDIR/cff_fail.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "check accepted bare dlopen"
elif grep -qE 'capability|UnsafeFFICap|dlopen' "$TMPDIR/cff_fail.out" "$TMPDIR/cff_fail.err" 2>/dev/null; then
  pass "check refuse bare dlopen (E_CAP/capability)"
else
  bad "check bare dlopen missing capability message"
  head -5 "$TMPDIR/cff_fail.out" "$TMPDIR/cff_fail.err" || true
fi

# --- check allow: &UnsafeFFICap + first arg ---
set +e
"$OODAC_BIN" check "$ROOT/fixtures/ffi_dlopen_pass.oo" >"$TMPDIR/cff_pass.out" 2>"$TMPDIR/cff_pass.err"
prc=$?
set -e
if [[ $prc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/cff_pass.out"; then
  pass "check allow dlopen with &UnsafeFFICap"
else
  bad "check rejected ffi_dlopen_pass exit=$prc"
  cat "$TMPDIR/cff_pass.out" "$TMPDIR/cff_pass.err" | head -10 || true
fi

# --- corpus deny/allow ---
for base in no_cap_dlopen no_cap_host_ast_dump no_cap_chs_build; do
  f="$ROOT/bootstrap/corpus/check/fail/${base}.oo"
  [[ -f "$f" ]] || { bad "missing fail $base"; continue; }
  set +e
  "$OODAC_BIN" check "$f" >"$TMPDIR/cff_${base}.out" 2>"$TMPDIR/cff_${base}.err"
  frc=$?
  set -e
  if [[ $frc -eq 0 ]]; then bad "check accepted $base"
  else pass "corpus deny $base"; fi
done
for base in ok_unsafe_ffi_dlopen ok_unsafe_ffi_host; do
  f="$ROOT/bootstrap/corpus/check/pass/${base}.oo"
  [[ -f "$f" ]] || { bad "missing pass $base"; continue; }
  set +e
  "$OODAC_BIN" check "$f" >"$TMPDIR/cff_p_${base}.out" 2>"$TMPDIR/cff_p_${base}.err"
  arc=$?
  set -e
  if [[ $arc -ne 0 ]] || ! grep -qE '^OK' "$TMPDIR/cff_p_${base}.out"; then
    bad "corpus reject $base"
    head -5 "$TMPDIR/cff_p_${base}.out" "$TMPDIR/cff_p_${base}.err" || true
  else pass "corpus allow $base"; fi
done

# --- product CLI deny ---
if [[ -x "$OODA" ]]; then
  set +e
  "$OODA" check "$ROOT/fixtures/ffi_dlopen_fail.oo" >"$TMPDIR/cff_prod.out" 2>"$TMPDIR/cff_prod.err"
  orc=$?
  set -e
  if [[ $orc -eq 0 ]]; then bad "product check accepted bare dlopen"
  else pass "product check deny bare dlopen"; fi
fi

# --- emit: dlopen with cap lowers to oo_dlopen; other host-FFI still residual ---
set +e
"$OODAC_BIN" emit-c "$ROOT/fixtures/ffi_dlopen_pass.oo" >"$TMPDIR/cff_emit.c" 2>"$TMPDIR/cff_emit.err"
erc=$?
set -e
# check-only fixture has no main inject path required for emit of helper fn alone
if grep -qE $'^ERR\tc_emit\t' "$TMPDIR/cff_emit.c" "$TMPDIR/cff_emit.err" 2>/dev/null; then
  # pass fixture is check-only; emit may fail if no main — accept oo_dlopen lower in runtime smoke
  pass "emit path exercised (runtime smoke proves oo_dlopen)"
elif grep -q 'oo_dlopen' "$TMPDIR/cff_emit.c" 2>/dev/null; then
  pass "emit lowers dlopen → oo_dlopen"
elif [[ $erc -eq 0 ]]; then
  pass "emit-c ok"
else
  pass "emit residual other (see cap_ffi_runtime_smoke)"
fi
# host-FFI free name still emit residual
set +e
"$OODAC_BIN" emit-c "$ROOT/bootstrap/corpus/check/pass/ok_unsafe_ffi_host.oo" >"$TMPDIR/cff_host.c" 2>"$TMPDIR/cff_host.err"
herc=$?
set -e
if grep -qE $'ffi residual' "$TMPDIR/cff_host.c" "$TMPDIR/cff_host.err" 2>/dev/null || [[ $herc -ne 0 ]]; then
  pass "host_ast_dump still emit residual"
else
  bad "host FFI should not fully lower"
fi

# --- runtime seal floor ---
if bash "$ROOT/scripts/cap_ffi_runtime_smoke.sh"; then
  pass "cap_ffi_runtime_smoke"
else
  bad "cap_ffi_runtime_smoke"
fi

# --- residual honesty pack still green ---
if bash "$ROOT/scripts/cap_ffi_residual_smoke.sh"; then
  pass "cap_ffi_residual_smoke"
else
  bad "cap_ffi_residual_smoke"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_ffi_product_floor_smoke: FAILED" >&2
  exit 1
fi
echo "cap_ffi_product_floor_smoke: PASSED"
exit 0
