#!/usr/bin/env bash
# job: native Backend-C contracts smoke (pass + fail + simple requires runtime)
# stage: test
# residual: ensures + complex requires not lowered; simple requires IDENT OP lit|ident are runtime
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

PASS="$ROOT/bootstrap/corpus/emit-c/pass/fn_contracts_add.oo"
FAIL="$ROOT/bootstrap/corpus/emit-c/fail/contract_no_brace.oo"
# Real fixtures that emptied bodies without contract skip:
IM="$ROOT/fixtures/int_main.oo"
HELLO="$ROOT/fixtures/hello.oo"

emit_ok() {
  local src="$1" base c_out
  base="$(basename "$src" .oo)"
  c_out="$TMPDIR/contracts_${base}.c"
  set +e
  "$OODAC" emit-c "$src" >"$c_out" 2>"$TMPDIR/contracts_${base}.err"
  local rc=$?
  set -e
  if [[ "$rc" != "0" ]]; then
    echo "FAIL contracts emit-c exit $rc: $src" >&2
    cat "$TMPDIR/contracts_${base}.err" >&2
    exit 1
  fi
  if grep -E $'^ERR\t' "$c_out" >/dev/null 2>&1; then
    echo "FAIL contracts emit-c ERR: $src" >&2
    grep -E $'^ERR\t' "$c_out" >&2 || true
    exit 1
  fi
  grep -v '🚀\|Running main' "$c_out" >"${c_out}.clean" || true
  mv "${c_out}.clean" "$c_out"
  # Non-empty function bodies (contracts must not wipe stmts)
  if ! grep -qE 'return |oo_print|oo_str_concat' "$c_out"; then
    echo "FAIL contracts empty body: $src" >&2
    head -80 "$c_out" >&2
    exit 1
  fi
  printf '%s' "$c_out"
}

# --- pass: emit + gcc + run (int path) ---
for src in "$PASS" "$IM"; do
  base="$(basename "$src" .oo)"
  c_out="$(emit_ok "$src")"
  bin_out="$TMPDIR/contracts_${base}.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_out" -o "$bin_out" -lm
  out="$(timeout 3 "$bin_out")"
  echo "$out" | grep -q '42' || {
    echo "FAIL contracts run expected 42: $src got: $out" >&2
    exit 1
  }
  echo "OK contracts pass $base"
done

# hello: emit non-empty greet body (let String ty residual may block gcc run)
c_hello="$(emit_ok "$HELLO")"
grep -q 'OoStr greet' "$c_hello" || {
  echo "FAIL hello missing greet: $HELLO" >&2
  exit 1
}
grep -A2 'OoStr greet' "$c_hello" | grep -q 'return' || {
  echo "FAIL hello empty greet body: $HELLO" >&2
  head -60 "$c_hello" >&2
  exit 1
}
echo "OK contracts pass hello (emit body)"

# --- fail: garbage / missing body after requires ---
c_fail="$TMPDIR/contracts_fail.c"
err_fail="$TMPDIR/contracts_fail.err"
set +e
"$OODAC" emit-c "$FAIL" >"$c_fail" 2>"$err_fail"
frc=$?
set -e
if grep -E $'^ERR\tc_emit' "$c_fail" "$err_fail" >/dev/null 2>&1; then
  echo "OK contracts fail contract_no_brace (ERR line)"
elif [[ "$frc" != "0" ]]; then
  echo "OK contracts fail contract_no_brace (exit $frc)"
else
  echo "FAIL contracts should reject: $FAIL" >&2
  head -30 "$c_fail" >&2
  exit 1
fi

# build path smoke (int_main)
rm -f /tmp/im_contracts
"$OODAC" build "$IM" /tmp/im_contracts
outb="$(/tmp/im_contracts)"
echo "$outb" | grep -q '42' || {
  echo "FAIL oodac build int_main: $outb" >&2
  exit 1
}
echo "OK contracts build int_main"

echo "contracts_native_smoke: pass+fail OK"
