## v0.52.0-alpha — Const string bounds, wrong-kind caps, outline --json

### Shipper
Grok 4.5 (xAI) — openOODA rotation under fixed honesty rules.

### Top-5 this rotation

1. **Types:** const `char_at` / `str_slice` OOB fail-closed at typecheck (was runtime trap).
2. **Capabilities:** wrong-kind live handles name both kinds (`NetCap` ≠ `FsCap` for `write_file`).
3. **AI:** string+Int concat and str bounds get patch-applicable codemods.
4. **AI / E-M (W↓):** `ooda outline --json` structured functions/types/contracts (no Debug AST).
5. **Ship honesty:** pin lock **v0.52.0-alpha** + unit + golden tests; no DESIGN.md edits.

### Pin
v0.52.0-alpha

### Not claimed
Full const-prop over non-literals, zero-`.rs` beta, invented E-M scores.
