# openOODA v0.100.0-alpha

Shipper: **Grok 4.5 (xAI)** — skeptic-fix after 5-cycle rotation.

## Fix (diff-proven)

1. **List[Int] `==`/`!=` uses `i32.eq`/`i32.ne` (header pointer identity)**, not `$streq` (string content). Distinct `list_new()` headers with equal elements compare **unequal**.
2. **Host e2e** `ooda_wasm_list_pointer_eq_not_streq`: `a==a` → 1, `a==b` → 0 for two lists each pushed `1`.
3. String `==` still uses `$streq` (content).

## E-M
- **D↓:** list equality no longer lies (false content equality via streq on headers).
- **W↓:** no extra heap; i32.eq is zero-cost vs host streq walk.
- **V↑:** correct dual-engine list identity for WASM subset.

## Pin
`v0.100.0-alpha`

## Not claimed
Full WASM product, content-deep list equality, zero-`.rs` beta.
