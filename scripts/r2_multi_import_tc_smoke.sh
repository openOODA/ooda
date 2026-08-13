#!/usr/bin/env bash
# job: R2 Multi-Import Type Check path-A smoke
# in:  OODAC_BIN (default oodac/oodac) + ephemeral TMPDIR fixtures
# out: exit 0 only if all pass/fail rails hold
#
# Rails:
#   PASS — 2-file import call (helper() across import)
#   PASS — struct type across import (type + make + field use)
#   FAIL — arg type mismatch across import (String into Int param)
#   FAIL — missing import fail-closed
#
# Residual (Issue #9): multi-import typecheck LINE:COL is expanded-stream
# offset after load_import concat, not per-file local. See oodac/tc_diag.oo.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

FIX="$TMPDIR/r2_multi_import_tc_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT

# --- fixtures (all under TMPDIR; never pollute repo root) ---

# PASS: 2-file import call
cat >"$FIX/call_lib.oo" <<'EOF'
// R2 path-A: exported helper for cross-file call
pub fn helper() -> Int {
    return 42;
}
EOF
cat >"$FIX/call_main.oo" <<'EOF'
import "call_lib.oo";
pub fn main() {
    let x: Int = helper();
    println(x);
}
EOF

# PASS: struct type across import
cat >"$FIX/struct_lib.oo" <<'EOF'
// R2 path-A: struct type + ctor + field consumer
type Point = struct {
    x: Int,
    y: Int,
};
pub fn make_point(a: Int, b: Int) -> Point {
    return Point { x: a, y: b };
}
pub fn point_sum(p: Point) -> Int {
    return p.x + p.y;
}
EOF
cat >"$FIX/struct_main.oo" <<'EOF'
import "struct_lib.oo";
pub fn main() {
    let p: Point = make_point(1, 2);
    let s: Int = point_sum(p);
    println(s);
}
EOF

# FAIL: arg type mismatch across import
cat >"$FIX/arg_lib.oo" <<'EOF'
// R2 path-A: Int param for cross-file arg type check
pub fn need_int(n: Int) -> Int {
    return n + 1;
}
EOF
cat >"$FIX/arg_bad_main.oo" <<'EOF'
import "arg_lib.oo";
pub fn main() {
    let x: Int = need_int("nope");
    println(x);
}
EOF

# FAIL: missing import fail-closed
cat >"$FIX/missing_main.oo" <<'EOF'
import "no_such_module.oo";
pub fn main() {
    println("unreachable");
}
EOF

# FAIL: arity mismatch across import
cat >"$FIX/arity_lib.oo" <<'EOF'
pub fn add(a: Int, b: Int) -> Int {
    return a + b;
}
EOF
cat >"$FIX/arity_bad_main.oo" <<'EOF'
import "arity_lib.oo";
pub fn main() {
    let x: Int = add(1);
    println(x);
}
EOF

# FAIL: multi-arg call ret type vs let ann across import (was silent OK)
cat >"$FIX/ret_lib.oo" <<'EOF'
pub fn add(a: Int, b: Int) -> Int {
    return a + b;
}
EOF
cat >"$FIX/ret_bad_main.oo" <<'EOF'
import "ret_lib.oo";
pub fn main() {
    let s: String = add(1, 2);
    println(s);
}
EOF

# PASS: chain import (mid re-exports leaf free name via concat)
cat >"$FIX/chain_lib.oo" <<'EOF'
pub fn leaf(x: Int) -> Int {
    return x + 1;
}
EOF
cat >"$FIX/chain_mid.oo" <<'EOF'
import "chain_lib.oo";
pub fn mid(x: Int) -> Int {
    return leaf(x);
}
EOF
cat >"$FIX/chain_main.oo" <<'EOF'
import "chain_mid.oo";
pub fn main() {
    let a: Int = mid(1);
    let b: Int = leaf(2);
    println(a);
    println(b);
}
EOF

# FAIL: multi-arg call used as arg with wrong expected type
cat >"$FIX/nest_lib.oo" <<'EOF'
pub fn add(a: Int, b: Int) -> Int {
    return a + b;
}
pub fn need_string(s: String) -> String {
    return s;
}
EOF
cat >"$FIX/nest_bad_main.oo" <<'EOF'
import "nest_lib.oo";
pub fn main() {
    let x: String = need_string(add(1, 2));
    println(x);
}
EOF

# --- helpers ---

check_ok() {
  local f="$1"
  local label="$2"
  local out="$FIX/${label}.out"
  local err="$FIX/${label}.err"
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
  if grep -qE $'^ERR\t' "$out" "$err" 2>/dev/null; then
    bad "pass rail has ERR: $label"
    return 1
  fi
  pass "pass: $label"
  return 0
}

check_err() {
  local f="$1"
  local label="$2"
  local want_re="$3"
  local out="$FIX/${label}.out"
  local err="$FIX/${label}.err"
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
  if ! echo "$blob" | grep -qE "$want_re"; then
    bad "fail rail no match /$want_re/: $label (got: $(echo "$blob" | head -2))"
    return 1
  fi
  pass "fail-closed: $label"
  return 0
}

# --- rails ---

check_ok "$FIX/call_main.oo" "2file_import_call" || true
check_ok "$FIX/struct_main.oo" "struct_type_across_import" || true
check_ok "$FIX/chain_main.oo" "chain_import_3file" || true
check_err "$FIX/arg_bad_main.oo" "arg_type_mismatch_across_import" \
  'ERR[[:space:]]+type|Type error|expects Int|got String' || true
check_err "$FIX/missing_main.oo" "missing_import_fail_closed" \
  'ERR[[:space:]]+import|ERR_IMPORT_MISSING|not found|missing' || true
check_err "$FIX/arity_bad_main.oo" "arity_mismatch_across_import" \
  'ERR[[:space:]]+type|expects 2|got 1|argument' || true
check_err "$FIX/ret_bad_main.oo" "multiarg_ret_type_across_import" \
  'ERR[[:space:]]+type|annotated as String|initializer has type Int|Type error' || true
check_err "$FIX/nest_bad_main.oo" "multiarg_call_as_arg_type" \
  'ERR[[:space:]]+type|expects String|got Int|Type error' || true

if [[ $fail -ne 0 ]]; then
  echo "r2_multi_import_tc_smoke: FAILED" >&2
  exit 1
fi
echo "r2_multi_import_tc_smoke: ALL OK"
echo "RESIDUAL Issue #9: multi-import expanded LINE offsets (load_import concat stream, not per-file local) — see oodac/tc_diag.oo / load_import.oo"
echo "RESIDUAL modular: names_to_depth0_binds ready; product check still whole-tree concat TC (small graphs)"
exit 0
