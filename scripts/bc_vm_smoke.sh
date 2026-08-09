#!/usr/bin/env bash
# job: M6 bytecode VM smoke — emit-bc + run (interpreter, not JIT)
# in:  oodac/oodac (or OODAC_BIN); optional bin/ooda; fixtures that BC subset supports
# out: exit 0 if ≥2 distinct fixtures emit-bc + run with asserted output
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

# Inline string fixture (println literal — BC subset; no let/fn residual)
printf 'pub fn main() {\n  println("Hello World");\n}\n' >"$TMP/hello_str.oo"

# Prefer existing fixtures when emit-bc + oodac run already support them.
# Each entry: name  src  expected_stdout  emit_bc_grep (ERE)
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

if [[ $fail -ne 0 ]]; then
  echo "bc_vm_smoke: FAILED" >&2
  exit 1
fi
if [[ $n -lt 2 ]]; then
  echo "bc_vm_smoke: need ≥2 fixtures, got $n" >&2
  exit 1
fi

echo "bc_vm_smoke: PASSED ($n fixtures; bytecode interpreter — not JIT)"
exit 0
