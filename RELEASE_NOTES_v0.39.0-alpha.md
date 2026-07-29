## v0.39.0-alpha — Fail-closed types, object-cap free sealed ops, dual-engine honesty

### E-M status (honest)
`ooda em` remains **measured clocks only** (parse/cap/typecheck µs, W, V = W/time, 1/latency).
No Boyd Ps fabrication, no 82.4% savings, no OPTIMAL MANEUVERABILITY floor.
`bench --em` now **prints the same measured report** before the empirical suite (was a dead flag).
`check_failed` still marks rework drag (D > 0) when cap/typecheck fails.

### Top-5 closed this rotation

1. **Types:** unifier fail-closed — `Unknown` is not a wildcard (only ADT holes inside Result/Option/List); `Custom` matches by name only; `Int` vs `String` returns reject.
2. **Capabilities:** free sealed builtins require a **live handle argument** (object-cap). Ambient `fetch(url)` with unused `&NetCap` param is denied; use `fetch(net, url)` / `write_file(fs, …)`.
3. **Dual engine:** compile targets refuse sealed I/O (no runtime cap tokens in C/LLVM/WASM); IR-only native link fails non-zero; `--release` fails closed (not a silent no-op).
4. **AI diagnostics:** cap `suggested_fix` emits **ooda patch JSON** with `applicability: "patch"` (not a fake function body).
5. **Refinements:** const-fold simple int expressions (`5+6`) for `Int[lo..hi]` bounds on init/assign/return.

### Pin
v0.39.0-alpha — Cargo, clap, CANONICAL, BOOTSTRAP_PIN, install.oo, README, docs, QA, website.

### Not claimed
True full object-cap at runtime in native binaries (compile refuses sealed I/O instead), full polymorphic ADT inference, real Boyd T/D forces, zero-`.rs` beta, full WASM product.
