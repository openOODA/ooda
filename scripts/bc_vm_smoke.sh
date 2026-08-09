#!/usr/bin/env bash
# job: M6 bytecode VM smoke — emit-bc + run (interpreter, not JIT)
# in:  oodac/oodac (or OODAC_BIN); optional bin/ooda; fixtures that BC subset supports
# out: exit 0 if ≥3 distinct fixtures emit-bc + run with asserted output
#
# Language surface proven on the bytecode interpreter (emit-bc + oodac run):
#   - println(int) / println(string)
#   - int binops (+ * with precedence), unary ! / -
#   - let / let mut, local load/store, assignment (`=` token kind EQ)
#   - while loops (LABEL / JUMP / JUMP_IF_FALSE)
#   - if-expression value form (if/else if/else as RHS of let) — single-expr blocks
#   - for-range `for i in lo..hi` (desugars to while-like JUMP loop)
#   - unary ! , compare (>/<), && , nested for
#
# Residual (honest, not claimed green here):
#   - multi-statement value blocks in if-expr keep first expr only
#   - match / struct / list / string method surface not smoked
#   - product `bin/ooda run` may still be Backend-C build+exec (not always BC VM);
#     when present it is checked for output parity only
#   - never claim JIT — this path is a stack bytecode interpreter only
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
OODA="${OODA:-$ROOT/bin/ooda}"
TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/bc_vm_smoke_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: $OODAC" >&2
  exit 1
fi

# Inline string fixture (println literal — BC subset)
printf 'pub fn main() {\n  println("Hello World");\n}\n' >"$TMP/hello_str.oo"
# Inline int arithmetic (binops + precedence; no let)
printf 'pub fn main() {\n  println(2 + 3 * 4);\n}\n' >"$TMP/arith_int.oo"
# Inline while + let mut + assign (smallest loop surface)
printf 'pub fn main() {\n  let mut i = 0;\n  while i < 3 {\n    i = i + 1;\n  }\n  println(i);\n}\n' >"$TMP/while_simple.oo"

n=0
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

run_fixture() {
  local name="$1" src="$2" expect="$3" grep_pat="$4"
  local bc="$TMP/${name}.bc" err="$TMP/${name}.err" out

  if [[ ! -f "$src" ]]; then
    bad "$name: missing source $src"
    return
  fi

  echo "== emit-bc $name =="
  set +e
  timeout 20 "$OODAC" emit-bc "$src" >"$bc" 2>"$err"
  local ec=$?
  set -e
  if [[ $ec -ne 0 ]]; then
    bad "emit-bc $name exit=$ec"
    cat "$err" >&2 || true
    return
  fi
  if ! grep -qE "$grep_pat" "$bc"; then
    bad "emit-bc $name missing expected ops ($grep_pat)"
    cat "$bc" >&2 || true
    return
  fi
  pass "emit-bc $name"

  echo "== oodac run $name =="
  set +e
  out=$(timeout 20 "$OODAC" run "$src" 2>&1)
  ec=$?
  set -e
  if [[ $ec -ne 0 ]]; then
    bad "oodac run $name exit=$ec out=$(printf '%q' "$out")"
    return
  fi
  if [[ "$out" != "$expect" ]]; then
    bad "oodac run $name output want $(printf '%q' "$expect") got $(printf '%q' "$out")"
    return
  fi
  pass "oodac run $name (interpreter)"

  if [[ -x "$OODA" ]]; then
    echo "== product ooda run $name =="
    set +e
    out=$(timeout 20 "$OODA" run "$src" 2>&1)
    ec=$?
    set -e
    if [[ $ec -ne 0 ]]; then
      bad "product run $name exit=$ec out=$(printf '%q' "$out")"
      return
    fi
    if [[ "$out" != "$expect" ]]; then
      bad "product run $name output want $(printf '%q' "$expect") got $(printf '%q' "$out")"
      return
    fi
    pass "product ooda run $name"
  fi

  n=$((n + 1))
}

# 1) existing int println fixture
run_fixture "chs_hello" "$ROOT/fixtures/chs_hello.oo" "1" '\.func main|PUSH_INT 1|CALL println'
# 2) string println (inline; distinct from int-only hello)
run_fixture "hello_str" "$TMP/hello_str.oo" "Hello World" '\.func main|PUSH_STR Hello World|CALL println'
# 3) int arithmetic with precedence (real language binops on VM)
run_fixture "arith_int" "$TMP/arith_int.oo" "14" 'PUSH_INT 2|PUSH_INT 3|PUSH_INT 4|MUL|ADD|CALL println'
# 4) while + let mut + assign (STORE_LOCAL / JUMP_IF_FALSE loop)
run_fixture "while_simple" "$TMP/while_simple.oo" "3" 'STORE_LOCAL|LABEL|JUMP_IF_FALSE|LOAD_LOCAL'
# 5) full fixtures/while_count.oo — while + unary ! + if-expr else-if (language surface)
run_fixture "while_count" "$ROOT/fixtures/while_count.oo" "3" 'JUMP_IF_FALSE|STORE_LOCAL|CALL println'
# 6) for-range sum 0..5 → 10 (BC desugar already in bc_emit_stmt)
run_fixture "for_range" "$ROOT/fixtures/for_range.oo" "10" 'PUSH_INT 0|PUSH_INT 5|LT|JUMP_IF_FALSE|ADD|STORE_LOCAL'
# 7) unary ! + if
run_fixture "bc_unary_not" "$ROOT/fixtures/bc_unary_not.oo" "1" 'NOT|EQ|JUMP_IF_FALSE|CALL println'
# 8) compare + && 
run_fixture "bc_compare_logic" "$ROOT/fixtures/bc_compare_logic.oo" "1" 'GT|LT|AND|JUMP_IF_FALSE'
# 9) nested for-range 3*2 → 6
run_fixture "bc_nested_for" "$ROOT/fixtures/bc_nested_for.oo" "6" 'PUSH_INT 3|PUSH_INT 2|LT|JUMP_IF_FALSE'
# 10) SUB/DIV — (20/4)-1 = 4 (no % — lexer residual)
run_fixture "bc_arith_ops" "$ROOT/fixtures/bc_arith_ops.oo" "4" 'DIV|SUB|CALL println'

if [[ $fail -ne 0 ]]; then
  echo "bc_vm_smoke: FAILED" >&2
  exit 1
fi
if [[ $n -lt 3 ]]; then
  echo "bc_vm_smoke: need ≥3 fixtures, got $n" >&2
  exit 1
fi

echo "bc_vm_smoke: PASSED ($n fixtures; bytecode interpreter — not JIT)"
exit 0
