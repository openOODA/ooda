## v0.28.0-alpha — QA honesty depth, fuzz fail-closed, no fake E-M telemetry

### Top-5 this rotation (DESIGN pillars)

1. **Capabilities / honesty:** high-value `sec_pen_*` suite now **executes** sealed `fetch` without `&NetCap` and **must trap**. Phase 4b expects TRAP (no empty-file greens).
2. **Contracts:** `ooda test --fuzz` **fails non-zero** on unexpected errors (postcondition/runtime), not soft-pass green checkmarks.
3. **AI diagnostics:** removed hardcoded `em_savings` / 82.4% theater from `--json-errors`. Golden asserts absence. Optional raw `timings_us` only when measured.
4. **Capabilities (FS):** `08_fs_capability.oo` real write/read round-trip under `&FsCap`.
5. **Version pin lock:** Cargo, clap, `install/BOOTSTRAP_PIN`, `install.oo`, website install defaults, README/QA headers → **v0.28.0-alpha**.

### Kept from prior alphas
- Real net `fetch` via curl, `where` fail-closed, python `Err`, old() postcondition snapshots
- CHS C backend, WASM Float subset, fail-closed LSP/pkg/migrate/replay

### Not claimed
LSP, pkg registry, AES, embedded CPython, full LLVM/WASM product, type-alias `where` bound checking, production E-M marketing scores.
