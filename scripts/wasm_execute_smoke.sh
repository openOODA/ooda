#!/usr/bin/env bash
# job: M4 WASM execute smoke — emit .wat AND run with wasmtime (prefer) or wasm3
# in:  ooda/oodac + host WASM runtime (wasmtime | wasm3)
# out: exit 0 only if emit + execute match expected stdout
# fail-closed: missing runtime → ERR_NO_WASMTIME (never soft-pass)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA_BIN:-${OODA:-$ROOT/bin/ooda}}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export OODA_SRC_ROOT="${OODA_SRC_ROOT:-$ROOT}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
TMP="$TMPDIR/wasm_execute_smoke_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

if [[ ! -x "$OODAC" && ! -x "$OODA" ]]; then
  echo "ERR_NO_COMPILER: need $OODAC or $OODA" >&2
  exit 1
fi

# Tiny string fixture — emits cleanly (direct println lit; avoid let/= residual)
# Expected WASI fd_write stdout (trim trailing whitespace/newlines for runtime variance)
EXPECTED="hi"
FIX_SRC="$TMP/hello_wasm.oo"
OUT_WAT="$TMP/hello_wasm.wat"
OUT_WASM="$TMP/hello_wasm.wasm"
cat >"$FIX_SRC" <<'EOF'
pub fn main() {
    println("hi")
}
EOF

echo "== M4 emit-wasm =="
set +e
# Prefer oodac emit-wasm (direct); ooda product path needs CWD/OODA_SRC_ROOT
if [[ -x "$OODAC" ]]; then
  timeout 30 "$OODAC" emit-wasm "$FIX_SRC" >"$OUT_WAT" 2>"$TMP/emit.err"
  ec=$?
else
  timeout 30 "$OODA" build --target wasm "$FIX_SRC" -o "$OUT_WAT" >"$TMP/emit.out" 2>"$TMP/emit.err"
  ec=$?
fi
set -e
if [[ $ec -ne 0 ]]; then
  echo "FAIL wasm emit exit=$ec" >&2
  cat "$TMP/emit.err" >&2 || true
  exit 1
fi
if [[ ! -s "$OUT_WAT" ]]; then
  echo "FAIL wasm emit: empty/missing .wat" >&2
  exit 1
fi
if ! grep -q 'module' "$OUT_WAT" \
  || ! grep -q 'wasi_snapshot_preview1' "$OUT_WAT" \
  || ! grep -q 'fd_write' "$OUT_WAT" \
  || ! grep -q '_start' "$OUT_WAT"; then
  echo "FAIL wasm emit: incomplete module shape" >&2
  head -40 "$OUT_WAT" >&2 || true
  exit 1
fi
if ! grep -q 'i32.const 1024' "$OUT_WAT" && ! grep -q 'call $println_str' "$OUT_WAT"; then
  echo "FAIL wasm emit: missing string/print lowering" >&2
  exit 1
fi
echo "OK emit-wasm ($(wc -c <"$OUT_WAT") bytes)"

# Prefer wasmtime; else wasm3 (+ wat2wasm if needed)
WASMTIME=""
WASM3=""
WAT2WASM=""
if command -v wasmtime >/dev/null 2>&1; then
  WASMTIME="$(command -v wasmtime)"
fi
if command -v wasm3 >/dev/null 2>&1; then
  WASM3="$(command -v wasm3)"
fi
if command -v wat2wasm >/dev/null 2>&1; then
  WAT2WASM="$(command -v wat2wasm)"
fi

if [[ -z "$WASMTIME" && -z "$WASM3" ]]; then
  echo "ERR_NO_WASMTIME: need wasmtime (prefer) or wasm3 on PATH to execute M4 WASM" >&2
  exit 1
fi

echo "== M4 execute =="
RUN_OUT="$TMP/run.out"
RUN_ERR="$TMP/run.err"
set +e
if [[ -n "$WASMTIME" ]]; then
  # wasmtime accepts .wat when text frontend is available
  timeout 15 "$WASMTIME" run --wasm-features=all "$OUT_WAT" >"$RUN_OUT" 2>"$RUN_ERR"
  ec=$?
  if [[ $ec -ne 0 ]]; then
    # fallback without feature flags / with explicit wasi
    timeout 15 "$WASMTIME" "$OUT_WAT" >"$RUN_OUT" 2>"$RUN_ERR"
    ec=$?
  fi
  RT_NAME="wasmtime"
else
  # wasm3 wants binary .wasm
  if [[ -n "$WAT2WASM" ]]; then
    "$WAT2WASM" "$OUT_WAT" -o "$OUT_WASM" 2>"$TMP/w2w.err"
    w2w_ec=$?
  else
    w2w_ec=1
    echo "ERR_NO_WASMTIME: wasm3 present but wat2wasm missing (cannot assemble .wat)" >&2
    exit 1
  fi
  if [[ $w2w_ec -ne 0 || ! -s "$OUT_WASM" ]]; then
    echo "FAIL wat2wasm" >&2
    cat "$TMP/w2w.err" >&2 || true
    exit 1
  fi
  timeout 15 "$WASM3" "$OUT_WASM" >"$RUN_OUT" 2>"$RUN_ERR"
  ec=$?
  RT_NAME="wasm3"
fi
set -e

if [[ $ec -ne 0 ]]; then
  echo "FAIL $RT_NAME execute exit=$ec" >&2
  cat "$RUN_ERR" >&2 || true
  cat "$RUN_OUT" >&2 || true
  exit 1
fi

got="$(tr -d '\r' <"$RUN_OUT" | sed -e 's/[[:space:]]*$//')"
if [[ "$got" != "$EXPECTED" ]]; then
  echo "FAIL wasm execute stdout mismatch (runtime=$RT_NAME)" >&2
  echo "  expected: $(printf '%q' "$EXPECTED")" >&2
  echo "  got:      $(printf '%q' "$got")" >&2
  exit 1
fi

echo "OK wasm execute via $RT_NAME (stdout matches expected)"
echo "wasm_execute_smoke: PASSED"
exit 0
