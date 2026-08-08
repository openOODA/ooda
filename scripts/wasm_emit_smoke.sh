#!/usr/bin/env bash
# job: smoke test for openOODA direct WebAssembly (.wat) backend
# stage: test
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA_BIN:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODA" && ! -x "$OODAC" ]]; then
  echo "ERR_NO_COMPILER: need $OODA or $OODAC" >&2
  exit 1
fi

TEST_SRC="$TMPDIR/wasm_smoke_test.oo"
OUT_WAT="$TMPDIR/wasm_smoke_test.wat"

cat << 'EOF' > "$TEST_SRC"
pub fn main() {
    let msg: String = "hello wasm smoke"
    println(msg)
}
EOF

if [[ -x "$OODA" ]]; then
  "$OODA" build --target wasm "$TEST_SRC" -o "$OUT_WAT"
else
  "$OODAC" emit-wasm "$TEST_SRC" > "$OUT_WAT"
fi

if [[ ! -f "$OUT_WAT" ]]; then
  echo "FAIL wasm_emit_smoke: output wat file not created" >&2
  exit 1
fi

if ! grep -q "module" "$OUT_WAT"; then
  echo "FAIL wasm_emit_smoke: missing module keyword" >&2
  exit 1
fi

if ! grep -q "wasi_snapshot_preview1" "$OUT_WAT"; then
  echo "FAIL wasm_emit_smoke: missing WASI preview1 import" >&2
  exit 1
fi

if ! grep -q "fd_write" "$OUT_WAT"; then
  echo "FAIL wasm_emit_smoke: missing fd_write import" >&2
  exit 1
fi

if ! grep -q "memory" "$OUT_WAT"; then
  echo "FAIL wasm_emit_smoke: missing memory declaration" >&2
  exit 1
fi

if ! grep -q '_start' "$OUT_WAT"; then
  echo "FAIL wasm_emit_smoke: missing _start export" >&2
  exit 1
fi

rm -f "$TEST_SRC" "$OUT_WAT"
echo "wasm_emit_smoke: WebAssembly backend smoke test OK"
