## v0.40.0-alpha — Runtime object-cap, em --json, monorepo pin lock

### Shipper
Grok 4.5 (xAI) — openOODA rotation under fixed honesty rules.

### Top-5 this rotation (DESIGN pillars)

1. **Capabilities (runtime):** free sealed builtins require a **live Capability value** in the arg list at runtime (not just ambient param declaration). Method-style receivers must be live handles. Matches static object-cap from v0.39.
2. **AI / E-M:** `ooda em --json` emits measured `EmReport` JSON (source_bytes, parse/cap/typecheck µs, throughput) — no theater fields.
3. **AI diagnostics:** cap `suggested_fix.applicability` is **`patch`** (ooda-patch JSON); golden asserts it.
4. **Dual engine:** golden test that `build --target c` on sealed FS programs fails non-zero (refuse without runtime tokens).
5. **Ship honesty:** monorepo `version_consistency` checks website `install` / `install.sh` pin when present; full pin lock → **v0.40.0-alpha**.

### Pin
v0.40.0-alpha — Cargo, clap, CANONICAL, BOOTSTRAP_PIN, install.oo, README, docs, QA, website install + install.sh.

### Not claimed
Native runtime caps in C binaries (still refused at compile), full Boyd T/D, zero-`.rs` beta, full WASM product, inventing E-M savings scores.
