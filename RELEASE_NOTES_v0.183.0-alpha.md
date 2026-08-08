# v0.183.0-alpha

**Not beta.** Owner-gated beta criteria live in `bootstrap/BETA.md`.

## Highlights

- **M1 AI tooling:** native `ooda outline` / `ooda reflect` (plain + `--json` / NDJSON); Python outline helpers removed; `outline_reflect_smoke.sh` green.
- **Self-host path:** `PURE_NO_ARC=1` (default in `bootstrap_no_cargo.sh`) strips seed-emitted ARC that heap-corrupts rebuilt `oodac`/`bin/ooda` until M2 ARC emit is fixed.
- **Backends (partial):** `oodac emit-llvm` → textual LLVM IR; `oodac emit-wasm` → WASI preview1 `.wat`; bytecode VM modules present (interpreter, not JIT).
- **Runtime:** non-consuming `oo_str_concat` (safe `s = s + t` with reassign ARC pattern).
- **Build infra:** pure_build function dedupe; string-aware pure_rewrite; `c_emit_ty_ext` split; line lock O=0.

## Build / install

```bash
export SEED_OODAC="${SEED_OODAC:-./bootstrap/seed/oodac}"
export PURE_NO_ARC="${PURE_NO_ARC:-1}"
export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-1}"
./scripts/bootstrap_no_cargo.sh
./bin/ooda version   # ooda 0.183.0-alpha (pure .oo CLI)
./scripts/outline_reflect_smoke.sh
```

Requires: bash, gcc, trusted seed binary. Does not require cargo/rustc.

## Pin

`install/BOOTSTRAP_PIN` = `v0.183.0-alpha`

## Residual honesty

- **M2 ARC:** seed-emitted retain/release still unsafe for self-host without `PURE_NO_ARC`. Post-release tree WIP: env last-wins + `c_emit_ret_arc` stash-before-release (see `bootstrap/ARC_M2_RESIDUAL.md`); stage2 ARC-on still deferred.
- **M3 fuzz:** product `--fuzz` path still uses Python harness (`ooda_fuzz_*.py`); `fuzz_gen.oo` not on product path.
- **M4–M6:** emit surfaces work; full clang/wasmtime/product rails and “JIT” claim not closed.
- **Seed:** cold start still needs prebuilt pure `oodac` once.
- Fail-closed / out of product: see `bootstrap/P4_DROPS.md` and `bootstrap/AUDIT_RESIDUAL.md`.
