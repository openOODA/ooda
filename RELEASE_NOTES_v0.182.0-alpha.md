# v0.182.0-alpha

**Not beta.** Owner-gated beta criteria live in `bootstrap/BETA.md`.

## Highlights

- **Zero Rust product tree** (B0): no `src/`, no `Cargo.toml`; pure `.oo` + C runtime + seed.
- **Pure product CLI** (`cli/main.oo` → `bin/ooda`) and pure `oodac` self-host.
- **Bootstrap without rustc:** `scripts/bootstrap_no_cargo.sh`, `scripts/ci_no_rust.sh`, `scripts/release.sh`.
- **Fixed-point** pure seed path (no OK_HOST soft-pass).
- **BETA.md** redesigned: Part A (B0–B5) + Part B (frozen In/Out surface); **only owner tags beta**.
- **Floor freedom:** `bootstrap/FLOOR.md`, measured `RUNTIME_ABI_v0.md`, `--backend c` allowlist (non-c fail-closed); F3 prep only.
- Residual fail-closed: wasm/llvm, `--json-errors`, `--fuzz`, non-c backends, host-FFI preamble decls.

## Build / install

```bash
export SEED_OODAC="${SEED_OODAC:-./oodac/oodac}"
./scripts/bootstrap_no_cargo.sh
./bin/ooda version   # ooda 0.182.0-alpha (pure .oo CLI)
```

Requires: bash, gcc, trusted seed binary. Does not require cargo/rustc.

## Pin

`install/BOOTSTRAP_PIN` = `v0.182.0-alpha`
