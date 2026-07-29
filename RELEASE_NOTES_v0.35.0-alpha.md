## v0.35.0-alpha — let-mut migrate, measured em playground, QA E-M phase

### Top-5 this rotation (DESIGN pillars)

1. **AI / migrate:** `ooda migrate --edition 2026` codemod #2 rewrites assigned immutable `let x` → `let mut x` (with tests; no double-mut).
2. **E-M honesty:** docs playground ships `em_demo.oo` with captured `ooda em` + `ooda run` output (measured clocks only — no fake drag-% / Boyd Ps theater).
3. **QA fail-closed:** Phase 5e runs `ooda em examples/em_demo.oo` and asserts measured labels present and theater strings absent.
4. **Ship honesty:** migrate let-mut surface was unreleased on v0.34 HEAD — now versioned under v0.35.0-alpha.
5. **Version pin lock:** Cargo, clap, BOOTSTRAP_PIN, install.oo, website install defaults, docs brand → **v0.35.0-alpha**.

### Pin
v0.35.0-alpha — Cargo, clap, BOOTSTRAP_PIN, install.oo, website install, docs brand.

### Not claimed
True object-cap (ambient grant still exists for free sealed ops), full native contract lowering, zero-`.rs` beta, full WASM product, real T/D forces for Boyd Ps.
