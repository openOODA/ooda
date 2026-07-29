## v0.44.0-alpha — Source-like fmt, fail-closed equality

### Shipper
Grok 4.5 (xAI) — openOODA rotation 5 under fixed honesty rules.

### Top-5 this rotation

1. **AI / E-M (W↓):** `ooda fmt` emits source-like syntax (requires/ensures/body) — no Debug AST dumps.
2. **Types:** `==` / `!=` fail-closed across incompatible types (`"a" == 1` rejects).
3. **Ship honesty:** pin lock **v0.44.0-alpha** across monorepo.
4. **Tests:** fmt unit tests + `rejects_string_eq_int`.
5. **No DESIGN.md** edits; unfinished CLIs remain fail-closed.

### Pin
v0.44.0-alpha

### Not claimed
Pretty-printer full fidelity for every AST node, zero-`.rs` beta, invented E-M scores.
