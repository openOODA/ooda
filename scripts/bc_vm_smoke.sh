#!/usr/bin/env bash
# job: M6 bytecode VM smoke — emit-bc + run (interpreter, not JIT)
# in:  oodac/oodac (or OODAC_BIN); optional bin/ooda
# out: exit 0 if hello works without residual
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

printf 'pub fn main() {\n  println("Hello World");\n}\n' >"$TMP/hello.oo"

echo "== emit-bc =="
set +e
timeout 20 "$OODAC" emit-bc "$TMP/hello.oo" >"$TMP/hello.bc" 2>"$TMP/hello.bc.err"
ec=$?
set -e
if [[ $ec -ne 0 ]]; then
  echo "FAIL emit-bc exit=$ec" >&2
  cat "$TMP/hello.bc.err" >&2 || true
  exit 1
fi
grep -qE '\.func main|CALL println|PUSH_STR' "$TMP/hello.bc" || {
  echo "FAIL emit-bc missing expected ops" >&2
  cat "$TMP/hello.bc" >&2
  exit 1
}
echo "OK emit-bc"

echo "== oodac run =="
set +e
out=$(timeout 20 "$OODAC" run "$TMP/hello.oo" 2>&1)
ec=$?
set -e
if [[ $ec -ne 0 ]] || ! echo "$out" | grep -q 'Hello World'; then
  echo "FAIL oodac run (ec=$ec out=$out)" >&2
  exit 1
fi
echo "OK oodac run (interpreter)"

if [[ -x "$OODA" ]]; then
  echo "== product ooda run =="
  set +e
  out2=$(timeout 20 "$OODA" run "$TMP/hello.oo" 2>&1)
  ec2=$?
  set -e
  if [[ $ec2 -ne 0 ]] || ! echo "$out2" | grep -q 'Hello World'; then
    echo "FAIL product run (ec=$ec2 out=$out2)" >&2
    exit 1
  fi
  echo "OK product ooda run"
fi

echo "bc_vm_smoke: PASSED (bytecode interpreter — not JIT)"
exit 0
