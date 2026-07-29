## v0.38.0-alpha — Nested refinements, nested/match cap aliases, pin honesty

### Top-5 this rotation (DESIGN pillars)

1. **Types / contracts:** `Int[lo..hi]` refinement bounds now apply inside nested
   `if` / `while` blocks (parent refinements inherited). Was a real hole: nested
   `port = 70000` silently typechecked.
2. **Capabilities:** cap-handle provenance is a fixed-point set over the full
   function AST — nested `if`/`while` let-aliases and `match Some(cap) { Some(h) => … }`
   pattern binds are live handles (not top-level-only).
3. **Honesty:** hollow `method_write_file_with_fscap_ok` test now asserts success
   (was a green no-op).
4. **Ship honesty:** empty v0.37 thrash (version-only bump claiming match/nested
   features) is replaced by real code + full pin lock → **v0.38.0-alpha**.
5. **E-M / AI surface:** no new fake telemetry; measured `ooda em` unchanged
   (clocks only). Pin lock restores trust in install path (cut D from pin drift).

### Pin
v0.38.0-alpha — Cargo, clap, CANONICAL_VERSION, BOOTSTRAP_PIN, install.oo, README,
docs brand, QA README, website install defaults.

### Not claimed
True object-cap (ambient grant for free sealed ops remains), full edition migrator,
full WASM product, zero-`.rs` beta, invented E-M savings / Boyd Ps scores,
“100% QA / N tests” theater.
