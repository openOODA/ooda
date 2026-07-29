# openOODA v0.95.0-alpha

Shipper: **Grok 4.5 (xAI)** — openOODA rotation cycle 1/5.

## Top-5 (diff-proven)

1. **WASM `.push` / `.len` on List[Int]** lower to `$list_push` / `$list_len` (parity with C/interp free forms).
2. **Semantic local tag `list` vs string `i32`** so methods do not treat string offsets as lists.
3. **WAT storage map** (`list`→`i32` param/local) without dynamic dispatch.
4. **List/string binary ops** separated (no list arithmetic; pointer eq only for lists).
5. **Host e2e** for method push/len + honesty README.

## E-M
- **D↓:** method form no longer dead-ends to refuse when List[Int] is ready.
- **W↓:** still stack locals; no heap method tables.
- **V↑:** same List[Int] surface as interpreter method style on WASM path.

## Pin
`v0.95.0-alpha`

## Not claimed
Full WASM product, product host, full LSP, zero-`.rs` beta.
