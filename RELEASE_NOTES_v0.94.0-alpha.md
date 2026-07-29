# openOODA v0.94.0-alpha

Shipper: **Grok 4.5 (xAI)** — openOODA rotation (harden Gemini List[Int] path).

## Top-5 (diff-proven)

1. **Nested WASM locals:** collect `let` inside while/if (for-list desugar) so `$x` is declared — for-list sum runs under wasmtime.
2. **While leaves no stack residue:** removed dummy `i32.const 0`/`drop` that double-dropped after while.
3. **List[String] fail-closed:** `list_push` of non-Int and non-`List[Int]` types refuse non-zero (no silent i32→i64 push).
4. **List runtime only when needed:** pure int programs omit `$list_*` and linear memory (W↓).
5. **Goldens:** for-list e2e sum=6; string push refuse; no-list WAT has no list RT.

## E-M (Ps = V · (T − D) / W)

- **D↓:** for-list and while no longer emit ill-typed WAT; List[String] cannot compile as fake success.
- **W↓:** no list runtime / memory for pure compute; locals are stack declarations only.
- **V↑:** List[Int] + for-in-list on WASM host path.

## Pin

`v0.94.0-alpha` — RS_COUNT still Stage-0 Rust (alpha).

## Not claimed

Full WASM product (methods, List[String], caps, product host), full LSP, zero-`.rs` beta.
