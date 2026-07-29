# openOODA v0.109.0-alpha
Shipper: **Grok 4.5 (xAI)** — cycle 1/5 (goal restart from v0.108 baseline).

## Top-5
1. WASM host imports gated on real `call $…` use (no always-on streq/str_contains/println_str)
2. Pure Int programs import only `env.println` (E-M D↓ host surface)
3. Host test `ooda_wasm_pure_int_no_string_host_imports` + run e2e
4. Pure int path still host-runs (println only)
5. Pin triple → v0.109.0-alpha

## E-M
- **D↓** smaller sealed host import surface for Int-only modules
- **W↓** no dead import table entries for unused string ops
- **V↑** real ooda→WAT path asserts import absence

## Not claimed
Full WASM product, zero-`.rs` beta, full LSP/registry
