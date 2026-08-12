#!/usr/bin/env bash
# CAP-G2 fixture harness — granular fs check (FsReadCap/FsWriteCap + FsCap legacy)
# Proves corpus under bootstrap/corpus/check/{fail,pass}/ via product oodac check.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

# Prefer product tree oodac; allow OODAC_BIN override; fall back to sibling oodac/
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
if [[ ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/../oodac/oodac" ]]; then
    OODAC="$ROOT/../oodac/oodac"
  else
    echo "ERR_NO_OODAC: need $ROOT/oodac/oodac" >&2
    exit 1
  fi
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

CHECK_PASS="$ROOT/bootstrap/corpus/check/pass"
CHECK_FAIL="$ROOT/bootstrap/corpus/check/fail"

# Static markers (soft honesty — source map + runtime require_fsread when present)
if grep -q 'sealed_fs_kind_of' "$ROOT/oodac/check_cap_util.oo" \
  && grep -q 'return "FsReadCap"' "$ROOT/oodac/check_cap_util.oo"; then
  pass "static sealed_fs_kind_of map (read_file→FsReadCap)"
else
  bad "missing sealed_fs_kind_of / FsReadCap preferred in check_cap_util.oo"
fi
if grep -q 'return "FsWriteCap"' "$ROOT/oodac/check_cap_util.oo" 2>/dev/null; then
  pass "static sealed_fs_kind_of map (write_file→FsWriteCap)"
else
  bad "missing write_file→FsWriteCap in sealed_fs_kind_of"
fi
if grep -qE 'oo_cap_require_fsread|require_fsread' "$ROOT/runtime/chs_rt_fs.c" \
  "$ROOT/runtime/chs_rt_sys.c" "$ROOT/runtime/chs_rt.h" 2>/dev/null; then
  pass "static runtime require_fsread / oo_cap_require_fsread"
else
  pass "static runtime require_fsread residual (not required for check harness)"
fi

expect_deny() { # $1=path $2=label
  local f="$1" label="$2" base out err rc
  base="$(basename "$f")"
  out="$TMPDIR/cap_g2_${base}.out"
  err="$TMPDIR/cap_g2_${base}.err"
  if [[ ! -f "$f" ]]; then bad "missing fixture $base"; return; fi
  set +e
  "$OODAC" check "$f" >"$out" 2>"$err"
  rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    bad "check accepted deny $label ($base)"
  elif ! grep -qiE 'capability|ERR|default-deny|cap' "$out" "$err" 2>/dev/null; then
    bad "check deny $label missing ERR ($base exit=$rc)"
    head -5 "$out" "$err" 2>/dev/null || true
  else
    pass "check deny $label"
  fi
}

expect_allow() { # $1=path $2=label
  local f="$1" label="$2" base out err rc
  base="$(basename "$f")"
  out="$TMPDIR/cap_g2_p_${base}.out"
  err="$TMPDIR/cap_g2_p_${base}.err"
  if [[ ! -f "$f" ]]; then bad "missing fixture $base"; return; fi
  set +e
  "$OODAC" check "$f" >"$out" 2>"$err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]] || ! grep -qE '^OK' "$out"; then
    bad "check rejected pass $label ($base exit=$rc)"
    head -8 "$out" "$err" 2>/dev/null || true
  else
    pass "check allow $label"
  fi
}

# fail: FsReadCap only + write_file
expect_deny "$CHECK_FAIL/wrong_granular_fsread_for_write.oo" "FsReadCap+write_file"
# fail: FsWriteCap only + read_file
expect_deny "$CHECK_FAIL/wrong_granular_fswrite_for_read.oo" "FsWriteCap+read_file"

# pass: FsCap + read_file + write_file (legacy supersede)
expect_allow "$CHECK_PASS/ok_fs_cap_read_write.oo" "FsCap+read+write"
# pass: FsReadCap + read_file
expect_allow "$CHECK_PASS/ok_fs_read_cap_read.oo" "FsReadCap+read_file"
# pass: FsWriteCap + write_file
expect_allow "$CHECK_PASS/ok_fs_write_cap_write.oo" "FsWriteCap+write_file"

# Legacy single-op FsCap fixtures still green
expect_allow "$CHECK_PASS/ok_fs_read.oo" "FsCap+read_file (legacy)"
expect_allow "$CHECK_PASS/ok_fs_write.oo" "FsCap+write_file (legacy)"

# Soft: named in ci_product rail list (do not fail product if not wired)
if grep -q 'cap_g2_fs_granular_smoke' "$ROOT/scripts/ci_product.sh" 2>/dev/null; then
  pass "ci_product soft-wire present"
else
  pass "ci_product soft-wire residual (not listed yet)"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_g2_fs_granular_smoke: FAILED" >&2
  exit 1
fi
echo "cap_g2_fs_granular_smoke: PASSED"
exit 0
