# openOODA v0.108.0-alpha
Shipper: **Grok 4.5 (xAI)** — skeptic fix (list-RT) republished after v0.107 tag immutability conflict.

## Top-5
1. `codegen_wasm::expr_needs_list`: `.len` only needs list RT when receiver is list-shaped
2. Variable String `.len` / `string_ops.oo` do not emit dead `$list_*` (W↓)
3. Host test `ooda_wasm_var_string_len_no_list_runtime` asserts `!$list_new`
4. Strengthened string-only / string_ops host tests for variable receivers
5. Pin triple → **v0.108.0-alpha** (v0.107 remote tag was deleted under immutable-release rules; forward-only)

## E-M
- **D↓** correct list-RT gate
- **W↓** pure string programs skip list heap RT
- **V↑** real `ooda build --target wasm` path

## Not claimed
Full WASM product, zero-`.rs` beta, full LSP/registry

## Note
v0.107.0-alpha was briefly created on the wrong remote commit by release-before-push; tag deleted and **forward-bumped** to v0.108 (cannot recreate immutable tag).
