# openOODA v0.107.0-alpha
Shipper: **Grok 4.5 (xAI)** — skeptic fix: list-RT injection on variable String.

## Top-5
1. `codegen_wasm::expr_needs_list`: `.len` only needs list RT when receiver is list-shaped (`list_new`/`list_push`/`.push`), not every non-literal
2. Variable String `.len` / `string_ops.oo` no longer emit dead `$list_new`/`$list_len`/… (W↓)
3. Host test `ooda_wasm_var_string_len_no_list_runtime` asserts `!$list_new`
4. Strengthened `ooda_wasm_string_only_no_list_runtime` + `string_ops` fixture tests for variable receivers
5. Pin triple → v0.107.0-alpha (tag = BOOTSTRAP = site)

## E-M
- **D↓** correct list-RT gate (no false dependency on list heap for strings)
- **W↓** pure string programs: memory for data segments only; no list header/heap RT
- **V↑** real `ooda build --target wasm` path covered by host e2e

## Not claimed
Full WASM product, zero-`.rs` beta, full LSP/registry

## Skeptic notes
Cycle-2 historical parallel `wasm_host` flake was fixed in v0.104 (`unique_temp_dir`). This ship is the list-RT honesty fix for variable String.
