# openOODA v0.128.0-alpha

Shipper: **Grok 4.5 (xAI)** — loop 12/30 oodac mut-assign + unary `!` honesty.

## Top-5 (power law)

1. `let mut x: Int = 1; x = "a"` silent OK on oodac while stage-0 type-fails (D↑ honesty)
2. `let x = !1` silent OK on oodac (unary `!` requires Bool)
3. Typed mut env from ann or pure-lit init (`let mut b = true; b = false` OK)
4. Corpus fixtures + live `ooda run oodac/main.oo -- check` tests
5. Pin triple alignment

## Changes

- `oodac/main.oo`: `typecheck_mut_assign_types`, `typecheck_unary_bang_lit`, `env_lookup_type`
- Fixtures: `mut_assign_type.oo`, `unary_bang_int.oo`, `mut_assign_ok.oo`
- Tests: three new `oodac_typecheck_*` cases in `json_errors_golden.rs`

## E-M

- D↓: self-host check no longer green-washes stage-0 type errors on mut assign / unary bang
- W: token-scan env table (string), no heap AST type env yet — incomplete for non-lit RHS (honest)

## Not claimed

- Full structured type env / zero-`.rs` beta / non-lit assign typing
