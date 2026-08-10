# v0.184.0-alpha

**Not beta.** Owner-gated beta criteria live in `bootstrap/BETA.md`.

## Highlights

- **Check cache (native `.oo`):** `oodac check` skips unchanged modules via
  `.ooda-cache/check` (tree + per-module fingerprints). Warm re-check of the
  full `oodac` graph is ~2s vs >120s full concat.
- **Build debt:** `oodac_pure_rewrite.py` **retired**. Capability grants
  (`oo_cap_grant_*`) and sealed call shapes come from native `c_emit` only
  (`pure_build: native_caps`).
- **M5 LLVM:** multi-binop IR closed (Pratt precedence: `2+3*4` → mul then add),
  `%` → `srem`, `%` restored in lex. `llvm_execute_smoke` proves clang execute
  for `42` and multi-binop `14` / mod `1`. Oversize llvm modules split for
  ≤256 line lock.
- **CLI version string:** `ooda 0.184.0-alpha (pure .oo CLI)`.

## Build / install

```bash
export SEED_OODAC="${SEED_OODAC:-./bootstrap/seed/oodac}"
export PURE_NO_ARC="${PURE_NO_ARC:-0}"
export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-1}"
./scripts/bootstrap_no_cargo.sh
./bin/ooda version   # ooda 0.184.0-alpha (pure .oo CLI)
source ~/.local/ooda-toolenv/env.sh   # clang/wasmtime for M4/M5 smokes
./scripts/llvm_execute_smoke.sh
./scripts/ci_product.sh
```

Requires: bash, gcc, trusted seed binary. Does not require cargo/rustc.
Optional for M4/M5 execute: toolenv with clang/wasmtime.

## Pin

`install/BOOTSTRAP_PIN` = `v0.184.0-alpha`

## Residual honesty

- **M2 ARC free:** reclaim still blocked for product self-host UAF; softener
  scripts residual; see `bootstrap/ARC_M2_RESIDUAL.md`.
- **Large-graph check:** modular path is parse + sealed walk; full multi-pass
  TC remains for small import graphs (≤12). Tree cache is the warm-path win.
- **LLVM floor:** emit + toolenv execute smoke only — **not** alternate
  self-host floor (Backend-C remains product floor; `P4_DROPS.md`).
- **M3 multi-type pure fuzz:** fail-closed residual.
- Fail-closed / out of product: see `bootstrap/P4_DROPS.md` and
  `bootstrap/AUDIT_RESIDUAL.md`.
