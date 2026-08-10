# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.183.0-alpha`**

OODA (Observe, Orient, Decide, Act) — capability-secure, self-testing, AI-native systems language.  
**Product path:** pure `.oo` compiler + CLI, thin C runtime floor, trusted seed binary (bash + gcc to rebuild).

> **DESIGN.md** is the north star (unchanged by alpha releases).  
> **Product purity (B0/B1):** zero `.rs` / no Cargo product build in this tree.  
> **Not a beta tag:** residual seed bootstrap, residual fail-closed features, org pin polish remain. See [`bootstrap/BETA.md`](bootstrap/BETA.md).

---

## Quick start

```bash
git clone https://github.com/openOODA/ooda.git && cd ooda
# Need a pure seed compiler once (prebuilt oodac, or existing oodac/oodac in tree):
export SEED_OODAC="${SEED_OODAC:-$PWD/oodac/oodac}"
./scripts/bootstrap_no_cargo.sh   # gcc + seed only → oodac/oodac + bin/ooda

./bin/ooda version
./bin/ooda check fixtures/chs_list_string.oo
./bin/ooda test fixtures/verify_pass.oo   # check + verify/assert_eq
./bin/ooda run fixtures/chs_list_string.oo   # native build+exec (permanent product path)
./bin/ooda dump tokens fixtures/int_main.oo
./bin/ooda build --target c fixtures/while_count.oo
# surgical edit (agents): replace function body only
# ./bin/ooda patch file.oo --replace-fn add --with body.txt [--check]
```

Requires: **bash, gcc, seed binary**. Pure product rebuild — no host language toolchain.

Install story (fetch prebuilt release + `install.oo`):

```bash
curl -fsSL https://openOODA.github.io/install.sh | sh
ooda version
```

---

## What's real in v0.183.0-alpha (pure product)

| Capability | Status |
|---|---|
| Product CLI `bin/ooda` | Pure `.oo` (`cli/main.oo`) → native; dispatches to pure `oodac` |
| Compiler `oodac` | Pure `.oo` self-host; lex/parse/check + emit-c + multi-module pure build |
| `check` / `dump tokens\|ast\|check` | Real on pure path |
| `build --target c\|chs\|native` | Real: emit-c + gcc + `runtime/chs_rt*.c` |
| `run` | **Permanent pure native build+exec** (no host interpreter; product path) |
| `test` | **Real:** check + run `verify`/`assert_eq!` via Backend-C harness; `--fuzz` un-gated **pure Int-domain** path (`ooda_fuzz_pure.sh` — no Python on that path; other domains fail closed) |
| `patch` | **Real:** structured `replace_fn` (CLI flags or JSON stdin); atomic write; path-safe |
| Fixed-point | `scripts/fixed_point.sh` pure seed → stage-1 → stage-2; digests s1≡s2; no OK_HOST; default **`PURE_NO_ARC=0`** (retain/release kept; runtime release leak-safe — see ARC residual) |
| Parity | `scripts/chs_parity.sh` product ≡ pure oodac |
| Line lock | `scripts/check_file_lines.sh` O=0 |
| Product purity | **B0/B1** — pure `.oo` tree (`RS_COUNT=0`; no `Cargo.toml` product path) |

### Residual / partial (not full product claims)

| Item | Behavior |
|---|---|
| `--json-errors` | **Real on check:** JSON diags + codes — see [`bootstrap/DIAG_CODES.md`](bootstrap/DIAG_CODES.md) |
| `--release` | Fail-closed residual (exit 2) |
| `--emit-llvm` / `build --target llvm` | **Emit + execute smoke** when `clang`/`llc` on PATH (`llvm_execute_smoke.sh`); still not a production optimize floor — see [`bootstrap/P4_DROPS.md`](bootstrap/P4_DROPS.md) |
| `build --target wasm` | **Emit + execute smoke** when `wasmtime`/`wasm3` on PATH (`wasm_execute_smoke.sh`); still not a full product WASM floor |
| `--fuzz` | **Un-gated** pure **Int-domain** path only (`// FUZZ_DOMAIN: int` markers) — see [`bootstrap/FUZZ_DEFER.md`](bootstrap/FUZZ_DEFER.md); do not claim full multi-type pure fuzzer |
| LSP / pkg / migrate / bench | Removed from product |
| Host interpreter | **Permanent product choice:** `ooda run` = native only (no interpret return) |
| contracts on native | **Simple `requires IDENT OP lit/ident` runtime-enforced** on Backend-C; **ensures** + complex requires still residual (not lowered); non-fuzz verify may still use Python harness |
| `verify` body | **`assert_eq!` / `assert_ne!` / `assert!`** lowered; other stmts fail-closed |
| `patch` line-range / AST node_id | Residual — `replace_fn` only |
| Cold-start seed | Need prebuilt pure `oodac` once (`SEED_OODAC`); bootstrap uses cold seed as emit host |
| Self-host ARC | Default **`PURE_NO_ARC=0`** (no strip); runtime **release does not free** (leak-safe) until reclaim is correct — [`bootstrap/ARC_M2_RESIDUAL.md`](bootstrap/ARC_M2_RESIDUAL.md) |
| Sealed caps (native) | Static check **plus** process-local **magic-token** runtime re-check for FS/Sys/Env; not cryptographic object-caps — see [`bootstrap/STATIC_CAPS.md`](bootstrap/STATIC_CAPS.md) |

---

## Floor / backends (freedom later)

Product self-host today uses **Backend-C** (`emit-c` + `runtime/chs_rt*` + `gcc`).  
That is an intentional thin OS floor under a pure `.oo` product — **not** required to stay the only floor forever.

- Policy + roadmap: [`bootstrap/FLOOR.md`](bootstrap/FLOOR.md)  
- Runtime ABI sketch: [`bootstrap/RUNTIME_ABI_v0.md`](bootstrap/RUNTIME_ABI_v0.md)  

Frontend (lex/parse/check) stays backend-neutral; lowering the floor means new emit/runtime/link packages, not rewriting the language.

## Bootstrap & release (pure product)

```bash
# Rebuild product from seed (pure .oo + gcc)
SEED_OODAC=./oodac/oodac ./scripts/bootstrap_no_cargo.sh

# Self-host referee
./scripts/fixed_point.sh

# Rails
./scripts/p3_no_cargo_smoke.sh
./scripts/product_pure_dispatch_smoke.sh
./scripts/chs_parity.sh
./scripts/beta_cli_smoke.sh
./scripts/c_emit_smoke.sh
./scripts/ci_product.sh   # product rails: seed bootstrap + smokes + fixed_point

# Release tarball (pure product pack)
./scripts/release.sh v0.183.0-alpha
```

Builder needs **gcc + seed binary only**. See `scripts/bootstrap_no_cargo.sh`.

---

## Beta gates (honest)

| Gate | Status (this pin) |
|---|---|
| **B0** product purity — no `.rs` | PASS |
| **B1** no Cargo product build | PASS local scripts; remote GHA residual (seed/private assets) |
| **B2** pure fixed-point surface | PASS under default `PURE_NO_ARC=0` (retain/release kept; leak-safe free residual) |
| **B3** ship pure `.oo`+C path | PASS path (`release.sh` + seed) |
| **B4** honesty / fail-closed residual | PASS process; **no beta tag** until public review |
| **B5** org siblings pure product path | PASS for product-critical siblings (std/qa/docs …); editors optional |

**Do not call this beta** until a public beta tag + install pin + notes are deliberately cut. This is **v0.183.0-alpha** pure `.oo` product.

Proof / status: monorepo `PROGRESS.md`; criteria `bootstrap/BETA.md`; latest ship notes `RELEASE_NOTES_v0.183.0-alpha.md`.

---

## Related

`spec`, `qa`, `std` (`.oo` only), `openOODA.github.io`, optional `tree-sitter` / `vscode` (editor support — not compiler critical path).

Ship notes: [GitHub Releases](https://github.com/openOODA/ooda/releases).
