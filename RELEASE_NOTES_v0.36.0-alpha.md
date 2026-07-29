## v0.36.0-alpha — Assign cap re-alias, assignment refinement bounds, pin lock cleanup

### What is actually new (vs v0.35)

1. **Capabilities:** `CapabilityChecker` traces sealed handles through **`Statement::Assign`**
   re-aliasing (`let mut fs_var = fs; fs_var = fs; fs_var.write_file(...)`), not only `let` aliases.
2. **Types:** integer refinement bounds `Int[lo..hi]` are checked on **assignment of int literals**,
   not only on `let` initializers (with unit test).
3. **Ship honesty:** full pin lock after thrash — Cargo, clap, `CANONICAL_VERSION`, BOOTSTRAP_PIN,
   install.oo, README, docs brand, QA README, website install defaults all **v0.36.0-alpha**.

### Already real (not new in 0.36 — do not re-market)

- Measured `ooda em` (parse/cap/typecheck µs, W, V) — no fake drag-% or Boyd Ps scores
- `ooda migrate` match wildcards + let→let mut (v0.35)
- Docs playground `em_demo` + QA Phase 5e (v0.35)
- Extended `ooda patch` body/params/return/contracts (v0.34)

### Pin
v0.36.0-alpha — Cargo, clap, BOOTSTRAP_PIN, install.oo, website install, docs brand, QA README.

### Not claimed
True object-cap (ambient grant still exists for free sealed ops), full native contract lowering,
zero-`.rs` beta, full WASM product, full edition migrator, invented E-M “savings” scores,
“100% QA / N tests” theater without a failing harness.
