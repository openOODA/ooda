#!/bin/bash
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$ROOT/oodac_tmp"
mkdir -p "$TMP"

if [ ! -f "$ROOT/oodac/oodac" ]; then
    echo "Compiler not found at $ROOT/oodac/oodac. Please build it first."
    exit 1
fi

cat << 'OO' > "$TMP/test_wasm_caps.oo"
fn main(fs: &FsCap, sys: &SysCap) {
    let a = 1;
}
OO

"$ROOT/oodac/oodac" emit-wasm "$TMP/test_wasm_caps.oo" > "$TMP/test_wasm_caps.wat"
~/.wasmtime/bin/wasmtime "$TMP/test_wasm_caps.wat" --invoke _start
echo "WASM CAPS SMOKE OK"
