# openOODA v0.93.0-alpha

Shipper: **Grok 4.5 (xAI)** — openOODA rotation.

## Top-5 (diff-proven)

1. **WASM `if` arms type-check:** side-effect-only branches (e.g. `println`) synthesize `i64.const 0` for `(result i64)`; statement-context exprs `drop` leftovers — wasmtime accepts modules that previously failed with empty stack.
2. **Dev-only wasmtime host smoke (real asserts):** capture `println`/`println_str`/`streq` output; assert `["hello","1","0"]`; **e2e** `ooda build --target wasm` → host run string `==`.
3. **Arg-type WorkspaceEdit whole-token:** walk back from call-site `)` (where typechecker points) to replace full `"hello"` / `true` / etc. — not first-character theater.
4. **Honesty:** `wasmtime` is **dev-dependency only** (not ship path / not beta product host); README updated.
5. **Pin v0.93.0-alpha** across Cargo, BOOTSTRAP_PIN, site install, GH Release. `RS_COUNT` remains Stage-0 Rust (alpha).

## E-M (Ps = V · (T − D) / W)

- **D↓:** invalid WAT no longer “succeeds” under structural-only validate then fails at host; arg quickfix no longer corrupts tokens.
- **W↓:** host smoke is test-only; product binary still no wasmtime. Branch zero is stack const (no heap).
- **V↑:** runnable WASM string-eq path under known host imports.

## Pin

`v0.93.0-alpha`

## Not claimed

Full WASM product, product WASM runtime, full LSP, registry, CPython embed, full self-host / zero `.rs` beta.
