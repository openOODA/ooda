## v0.41.0-alpha — AI type patches, QA object-cap/em-json, dual-engine wasm seal

### Shipper
Grok 4.5 (xAI) — openOODA rotation 2 under fixed honesty rules.

### Top-5 this rotation

1. **AI diagnostics:** high-frequency TypeErrors (`immutable`/`let mut`, must-use, non-exhaustive, undefined fn) emit `suggested_fix.applicability=patch` with ooda-patch / codemod JSON.
2. **Dual engine:** golden — `build --target wasm` refuses sealed FS I/O (same honesty as C).
3. **QA fail-closed:** Phase 5f `ooda em --json`; Phase 5g ambient-only `fetch` must deny (object-cap).
4. **Ship honesty:** monorepo docs README pin check when sibling present.
5. **E-M:** keep measured-only path; QA asserts em --json fields and no theater.

### Pin
v0.41.0-alpha — full lock across Cargo/clap/CANONICAL/BOOTSTRAP/install/README/docs/qa/site.

### Not claimed
Full auto-apply of type patches without human review, native runtime caps in C, zero-`.rs` beta, invented E-M scores.
