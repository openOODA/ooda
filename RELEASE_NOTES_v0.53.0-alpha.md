## v0.53.0-alpha — Nested scopes, param refinements, WASM while polarity

### Shipper
Grok 4.5 (xAI) — openOODA rotation under fixed honesty rules.

### Top-5 this rotation

1. **Types / runtime:** nested `let` no longer pollutes outer scopes (if/while) — type env + interpreter.
2. **Contracts/types:** `Int[lo..hi]` on **parameters** enforced at const call-sites (typecheck) and at runtime for non-const.
3. **Dual engine:** WASM `while` break polarity fixed (`i64.eqz` → break on false; was inverted).
4. **AI:** `RefinementTypeViolation` diagnostics emit patch-applicable `refinement_bounds` codemod.
5. **Ship honesty:** pin lock **v0.53.0-alpha** + unit/golden tests; website/docs/qa pins; no DESIGN.md edits.

### Pin
v0.53.0-alpha

### Not claimed
Full const-prop over non-literals for all refinements, zero-`.rs` beta, full WASM product, invented E-M scores.
