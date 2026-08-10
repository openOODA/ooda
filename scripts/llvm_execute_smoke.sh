#!/usr/bin/env bash
# job: M5 LLVM execute smoke — emit IR AND compile+run via clang (or llc+clang/gcc)
# in:  oodac + host LLVM tools (clang and/or llc) + runtime
# out: exit 0 only if emit + compile + run match expected stdout
# fail-closed: missing tools → ERR_NO_LLVM (never soft-pass)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA_BIN:-${OODA:-$ROOT/bin/ooda}}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export OODA_SRC_ROOT="${OODA_SRC_ROOT:-$ROOT}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
TMP="$TMPDIR/llvm_execute_smoke_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

if [[ ! -x "$OODAC" && ! -x "$OODA" ]]; then
  echo "ERR_NO_COMPILER: need $OODAC or $OODA" >&2
  exit 1
fi

# Tiny int fixture — emit-llvm currently lowers println(42) cleanly to oo_print_int
# Residual: multi-op binop IR has empty mul operands (2+3*4 not execute-proven)
EXPECTED="42"
FIX_SRC="$TMP/hello_llvm.oo"
OUT_LL="$TMP/hello_llvm.ll"
OUT_BIN="$TMP/hello_llvm.bin"
OUT_OBJ="$TMP/hello_llvm.o"
cat >"$FIX_SRC" <<'EOF'
pub fn main() {
    println(42)
}
EOF

echo "== M5 emit-llvm =="
set +e
if [[ -x "$OODAC" ]]; then
  timeout 30 "$OODAC" emit-llvm "$FIX_SRC" >"$OUT_LL" 2>"$TMP/emit.err"
  ec=$?
elif [[ -x "$OODA" ]]; then
  timeout 30 "$OODA" build --target llvm "$FIX_SRC" -o "$OUT_LL" >"$TMP/emit.out" 2>"$TMP/emit.err"
  ec=$?
else
  ec=1
fi
set -e
if [[ $ec -ne 0 ]]; then
  echo "FAIL emit-llvm exit=$ec" >&2
  cat "$TMP/emit.err" >&2 || true
  exit 1
fi
if [[ ! -s "$OUT_LL" ]]; then
  echo "FAIL emit-llvm: empty/missing .ll" >&2
  exit 1
fi
if ! grep -q 'define ' "$OUT_LL" || ! grep -q 'i64 42' "$OUT_LL"; then
  echo "FAIL emit-llvm: missing define / i64 42 constant" >&2
  head -80 "$OUT_LL" >&2 || true
  exit 1
fi
if ! grep -q 'oo_print_int' "$OUT_LL"; then
  echo "FAIL emit-llvm: missing oo_print_int call" >&2
  exit 1
fi
echo "OK emit-llvm ($(wc -c <"$OUT_LL") bytes)"

CLANG=""
LLC=""
if command -v clang >/dev/null 2>&1; then
  CLANG="$(command -v clang)"
fi
if command -v llc >/dev/null 2>&1; then
  LLC="$(command -v llc)"
fi
# linker for obj+runtime when using llc path
LINKER=""
if [[ -n "$CLANG" ]]; then
  LINKER="$CLANG"
elif command -v gcc >/dev/null 2>&1; then
  LINKER="$(command -v gcc)"
elif command -v cc >/dev/null 2>&1; then
  LINKER="$(command -v cc)"
fi

if [[ -z "$CLANG" && -z "$LLC" ]]; then
  echo "ERR_NO_LLVM: need clang (prefer) or llc on PATH to compile+run M5 IR" >&2
  exit 1
fi
if [[ -z "$CLANG" && -n "$LLC" && -z "$LINKER" ]]; then
  echo "ERR_NO_LLVM: llc present but no clang/gcc/cc to link object + runtime" >&2
  exit 1
fi

RT_C="$ROOT/runtime/chs_rt.c"
RT_I="$ROOT/runtime"
if [[ ! -f "$RT_C" ]]; then
  echo "ERR_NO_RUNTIME: missing $RT_C" >&2
  exit 1
fi

echo "== M5 compile+link =="
set +e
if [[ -n "$CLANG" ]]; then
  # clang consumes textual IR directly
  timeout 60 "$CLANG" -O0 -Wno-override-module -Wno-unused-command-line-argument \
    -I"$RT_I" "$OUT_LL" "$RT_C" -o "$OUT_BIN" -lm \
    >"$TMP/cc.out" 2>"$TMP/cc.err"
  cc_ec=$?
  CC_PATH="clang"
else
  timeout 60 "$LLC" -filetype=obj -o "$OUT_OBJ" "$OUT_LL" >"$TMP/llc.out" 2>"$TMP/llc.err"
  llc_ec=$?
  if [[ $llc_ec -ne 0 || ! -s "$OUT_OBJ" ]]; then
    echo "FAIL llc exit=$llc_ec" >&2
    cat "$TMP/llc.err" >&2 || true
    exit 1
  fi
  timeout 60 "$LINKER" -O0 -I"$RT_I" "$OUT_OBJ" "$RT_C" -o "$OUT_BIN" -lm \
    >"$TMP/cc.out" 2>"$TMP/cc.err"
  cc_ec=$?
  CC_PATH="llc+$LINKER"
fi
set -e

if [[ $cc_ec -ne 0 || ! -x "$OUT_BIN" ]]; then
  echo "FAIL llvm compile/link via $CC_PATH exit=$cc_ec" >&2
  cat "$TMP/cc.err" >&2 || true
  cat "$TMP/cc.out" >&2 || true
  exit 1
fi
echo "OK compile+link via $CC_PATH"

echo "== M5 run =="
set +e
timeout 5 "$OUT_BIN" >"$TMP/run.out" 2>"$TMP/run.err"
run_ec=$?
set -e
if [[ $run_ec -ne 0 ]]; then
  echo "FAIL llvm binary run exit=$run_ec" >&2
  cat "$TMP/run.err" >&2 || true
  exit 1
fi

got="$(tr -d '\r' <"$TMP/run.out" | sed -e 's/[[:space:]]*$//')"
if [[ "$got" != "$EXPECTED" ]]; then
  echo "FAIL llvm execute stdout mismatch" >&2
  echo "  expected: $(printf '%q' "$EXPECTED")" >&2
  echo "  got:      $(printf '%q' "$got")" >&2
  exit 1
fi

echo "OK llvm execute (stdout matches expected 42)"

# Multi-binop + modulo (historical residual: empty mul operands / missing %)
echo "== M5 multi-binop+mod =="
BINOP_SRC="$TMP/binop_llvm.oo"
BINOP_LL="$TMP/binop_llvm.ll"
BINOP_BIN="$TMP/binop_llvm.bin"
cat >"$BINOP_SRC" <<'EOF'
pub fn main(fs: &FsCap, sys: &SysCap, args: List[String]) {
    let x = 2 + 3 * 4;
    println(x.to_string());
    let w = 10 % 3;
    println(w.to_string());
}
EOF
set +e
timeout 30 "$OODAC" emit-llvm "$BINOP_SRC" >"$BINOP_LL" 2>"$TMP/binop_emit.err"
ec=$?
set -e
if [[ $ec -ne 0 || ! -s "$BINOP_LL" ]]; then
  echo "FAIL multi-binop emit-llvm" >&2
  cat "$TMP/binop_emit.err" >&2 || true
  exit 1
fi
if ! grep -q 'mul i64' "$BINOP_LL" || ! grep -q 'add i64' "$BINOP_LL"; then
  echo "FAIL multi-binop IR missing mul/add" >&2
  grep -E 'add|mul|srem' "$BINOP_LL" >&2 || true
  exit 1
fi
if ! grep -q 'srem i64' "$BINOP_LL"; then
  echo "FAIL multi-binop IR missing srem for %" >&2
  exit 1
fi
set +e
if [[ -n "${CLANG:-}" ]]; then
  timeout 60 "$CLANG" -O0 -Wno-override-module -Wno-unused-command-line-argument \
    -I"$RT_I" "$BINOP_LL" "$RT_C" -o "$BINOP_BIN" -lm \
    >"$TMP/binop_cc.out" 2>"$TMP/binop_cc.err"
  cc_ec=$?
else
  timeout 60 "$LLC" -filetype=obj -o "$TMP/binop.o" "$BINOP_LL" >"$TMP/binop_llc.err" 2>&1
  timeout 60 "$LINKER" -O0 -I"$RT_I" "$TMP/binop.o" "$RT_C" -o "$BINOP_BIN" -lm \
    >"$TMP/binop_cc.out" 2>"$TMP/binop_cc.err"
  cc_ec=$?
fi
set -e
if [[ $cc_ec -ne 0 || ! -x "$BINOP_BIN" ]]; then
  echo "FAIL multi-binop compile" >&2
  cat "$TMP/binop_cc.err" >&2 || true
  exit 1
fi
got2="$(timeout 5 "$BINOP_BIN" | tr -d '\r' | sed -e 's/[[:space:]]*$//')"
# 2+3*4=14, 10%3=1
if [[ "$got2" != $'14\n1' ]]; then
  echo "FAIL multi-binop execute stdout" >&2
  echo "  expected: 14\\n1" >&2
  echo "  got:      $(printf '%q' "$got2")" >&2
  exit 1
fi
echo "OK multi-binop+mod execute (14, 1)"

echo "llvm_execute_smoke: PASSED"
exit 0
