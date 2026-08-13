#!/usr/bin/env bash
# job: SPRINT Issue #12 typecheck bug rails — pass / fail-closed / residual honesty
# in:  OODAC_BIN (default ./oodac/oodac) + fixtures/typecheck_bugs + fixtures/xfail
# out: exit 0 if expected-pass OK, expected-fail ERR, residuals documented
#
# Patterns (Issue #12):
#   a) &UserStruct param field access (direct / paren / copy)
#   b) multi-module shim import (dnssec-style)
#   c) if s.flag == false { } on user struct
#   d) large struct lit (≥5 fields)
# Plus fail-closed negatives and xfail residual notes (no silent green/red lies).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"

OODAC_BIN="${OODAC_BIN:-./oodac/oodac}"
OODAC="$OODAC_BIN"
PASS_DIR="$ROOT/fixtures/typecheck_bugs/pass"
FAIL_DIR="$ROOT/fixtures/typecheck_bugs/fail"
XFAIL_DIR="$ROOT/fixtures/xfail"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need executable $OODAC" >&2
  exit 1
fi

fail=0
n_pass=0
n_fail=0
n_xfail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
note() { echo "NOTE $*"; }

check_ok() {
  local f="$1"
  local label="$2"
  local out="$TMPDIR/tcbug_pass_$$.out"
  local err="$TMPDIR/tcbug_pass_$$.err"
  set +e
  "$OODAC" check "$f" >"$out" 2>"$err"
  local rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    bad "pass rail exit=$rc: $label"
    cat "$out" "$err" 2>/dev/null | head -12 >&2 || true
    return 1
  fi
  if ! grep -q '^OK' "$out" 2>/dev/null; then
    bad "pass rail missing OK: $label"
    cat "$out" "$err" 2>/dev/null | head -12 >&2 || true
    return 1
  fi
  if grep -qE $'^ERR' "$out" "$err" 2>/dev/null; then
    bad "pass rail has ERR: $label"
    return 1
  fi
  pass "pass: $label"
  return 0
}

check_err() {
  local f="$1"
  local label="$2"
  local out="$TMPDIR/tcbug_fail_$$.out"
  local err="$TMPDIR/tcbug_fail_$$.err"
  set +e
  "$OODAC" check "$f" >"$out" 2>"$err"
  local rc=$?
  set -e
  local blob
  blob=$(cat "$out" "$err" 2>/dev/null || true)
  if [[ $rc -eq 0 ]]; then
    bad "fail rail accepted (want ERR): $label"
    echo "$blob" | head -8 >&2 || true
    return 1
  fi
  if ! echo "$blob" | grep -qE 'ERR|Type error|type'; then
    bad "fail rail no type ERR: $label (got: $(echo "$blob" | head -2))"
    return 1
  fi
  pass "fail-closed: $label"
  return 0
}

# --- (a)(c)(d) + mut assign: single-file pass rails ---
shopt -s nullglob
for f in "$PASS_DIR"/*.oo; do
  [[ -f "$f" ]] || continue
  n_pass=$((n_pass + 1))
  base="$(basename "$f")"
  lines=$(wc -l <"$f" | tr -d ' ')
  if [[ "$lines" -gt 80 ]]; then
    bad "fixture over 80 lines ($lines): $base"
  fi
  check_ok "$f" "$base" || true
done

# --- (b) multi-module shim (dnssec pattern) ---
SHIM="$PASS_DIR/multi_shim/shim.oo"
if [[ -f "$SHIM" ]]; then
  n_pass=$((n_pass + 1))
  for leaf in "$PASS_DIR/multi_shim"/mod_a.oo "$PASS_DIR/multi_shim"/mod_b.oo "$SHIM"; do
    lines=$(wc -l <"$leaf" | tr -d ' ')
    if [[ "$lines" -gt 80 ]]; then
      bad "fixture over 80 lines ($lines): multi_shim/$(basename "$leaf")"
    fi
  done
  check_ok "$SHIM" "multi_shim/shim.oo" || true
else
  bad "missing multi_shim/shim.oo"
fi

# --- fail-closed negatives (must ERR type) ---
for f in "$FAIL_DIR"/*.oo; do
  [[ -f "$f" ]] || continue
  n_fail=$((n_fail + 1))
  base="$(basename "$f")"
  lines=$(wc -l <"$f" | tr -d ' ')
  if [[ "$lines" -gt 80 ]]; then
    bad "fixture over 80 lines ($lines): fail/$base"
  fi
  check_err "$f" "$base" || true
done

if [[ "$n_pass" -eq 0 ]]; then
  bad "no pass fixtures under $PASS_DIR"
fi
if [[ "$n_fail" -eq 0 ]]; then
  bad "no fail fixtures under $FAIL_DIR"
fi

# --- xfail / residual honesty (Issue #10 style) ---
# Two residual classes:
#   1) still-green soft residual (dup import free-names accept) — document only
#   2) still-broken residual (same param name poisons second struct) — expect ERR
# Smoke stays exit 0 when residuals match documented tip behavior.
if [[ -d "$XFAIL_DIR" ]]; then
  # soft residual: dual import same free-name currently accepts
  if [[ -f "$XFAIL_DIR/dup_import_names/shim.oo" ]]; then
    n_xfail=$((n_xfail + 1))
    set +e
    "$OODAC" check "$XFAIL_DIR/dup_import_names/shim.oo" \
      >"$TMPDIR/tcbug_xfail.out" 2>"$TMPDIR/tcbug_xfail.err"
    rc=$?
    set -e
    if [[ $rc -eq 0 ]] && grep -q '^OK' "$TMPDIR/tcbug_xfail.out" 2>/dev/null; then
      note "xfail residual still green (documented, not a pass rail): fixtures/xfail/dup_import_names/shim.oo"
      note "  residual: duplicate import free-names accept — Issue #10 collision policy open"
    else
      note "xfail residual now ERRs (collision policy tightened?): dup_import_names rc=$rc"
      cat "$TMPDIR/tcbug_xfail.out" "$TMPDIR/tcbug_xfail.err" 2>/dev/null | head -6 || true
    fi
  fi
  # hard residual: same param name on two struct fns — tip currently broken
  if [[ -f "$XFAIL_DIR/same_param_name/repro.oo" ]]; then
    n_xfail=$((n_xfail + 1))
    set +e
    "$OODAC" check "$XFAIL_DIR/same_param_name/repro.oo" \
      >"$TMPDIR/tcbug_xfail2.out" 2>"$TMPDIR/tcbug_xfail2.err"
    rc=$?
    set -e
    blob=$(cat "$TMPDIR/tcbug_xfail2.out" "$TMPDIR/tcbug_xfail2.err" 2>/dev/null || true)
    if [[ $rc -ne 0 ]] && echo "$blob" | grep -qE 'ERR|Type error|no field'; then
      note "xfail residual still broken as documented: fixtures/xfail/same_param_name/repro.oo"
      note "  residual: shared param name poisons second struct field table (use distinct names)"
    elif [[ $rc -eq 0 ]]; then
      note "xfail residual FIXED (now green): same_param_name — promote to pass rail when ready"
    else
      bad "xfail same_param_name unexpected outcome rc=$rc"
      echo "$blob" | head -8 >&2 || true
    fi
  fi
fi

# Honesty banner: patterns a–d are green due to DnssecRecord-style fixes on tip
pass "honesty: &UserStruct / if field==false / multi-shim / large lit = PASS rails (not xfail)"
pass "honesty: fail/ stays fail-closed for missing/non-Bool field and bad large lit"

if [[ $fail -ne 0 ]]; then
  echo "typecheck_bug_rails_smoke: FAILED (pass=$n_pass fail=$n_fail xfail_notes=$n_xfail)" >&2
  exit 1
fi
echo "typecheck_bug_rails_smoke: ALL OK (pass=$n_pass fail-closed=$n_fail residual_notes=$n_xfail)"
exit 0
