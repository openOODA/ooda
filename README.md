# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.183.0-alpha`**

OODA (Observe, Orient, Decide, Act) — capability-secure, self-testing, AI-native systems language.

> **DESIGN.md** is the north star (unchanged by alpha releases).  
> Product tree is **zero `.rs`** (B0). Build/ship path uses pure `.oo` + C runtime + a trusted seed binary — **no Cargo/rustc**.  
> **Not a beta tag:** residual seed bootstrap, residual fail-closed features, org pin polish remain. See [`bootstrap/BETA.md`](bootstrap/BETA.md).

---

## Quick start (no Rust)

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

Requires: **bash, gcc, seed binary**. Does **not** require `cargo`, `rustc`, or `rustup`.

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
| `test` | **Real:** check + run `verify`/`assert_eq!` via Backend-C harness; `--fuzz` DESIGN-deferred |
| `patch` | **Real:** structured `replace_fn` (CLI flags or JSON stdin); atomic write; path-safe |
| Fixed-point | `scripts/fixed_point.sh` pure seed → stage-1 → stage-2; digests s1≡s2; no OK_HOST |
| Parity | `scripts/chs_parity.sh` product ≡ pure oodac |
| Line lock | `scripts/check_file_lines.sh` O=0 |
| Zero `.rs` in product tree | **B0** (`RS_COUNT=0`; no `src/`, no `Cargo.toml`) |

### Residual fail-closed (non-zero; not beta surface)

| Item | Behavior |
|---|---|
| `--json-errors` | **Real on check:** JSON diags + codes — see [`bootstrap/DIAG_CODES.md`](bootstrap/DIAG_CODES.md) |
| `--release` / `--emit-llvm` | Fail-closed residual |
| `--fuzz` | Fail-closed DESIGN deferral — see [`bootstrap/FUZZ_DEFER.md`](bootstrap/FUZZ_DEFER.md) |
| `build --target wasm\|llvm` | Fail-closed (beta-out / residual) |
| LSP / pkg / migrate / bench | Removed from product |
| Host interpreter | **Permanent product choice:** `ooda run` = native only (no interpret return) |
| contracts on native | **Simple `requires IDENT OP lit/ident` runtime-enforced** on Backend-C; **ensures** + complex requires still residual (not lowered); test harness strips contracts for verify harness |
| `verify` body | **`assert_eq!` / `assert_ne!` / `assert!`** lowered; other stmts fail-closed |
| `patch` line-range / AST node_id | Residual — `replace_fn` only |
| Cold-start seed | Need prebuilt pure `oodac` once (`SEED_OODAC`) |
| Sealed caps (native) | **Static check only** — no runtime re-check; Backend-C erases cap tokens to ambient libc — see [`bootstrap/STATIC_CAPS.md`](bootstrap/STATIC_CAPS.md) |

---

## Floor / backends (freedom later)

Product self-host today uses **Backend-C** (`emit-c` + `runtime/chs_rt*` + `gcc`).  
That is an intentional thin OS floor, **not** a Rust host and **not** required to stay the only floor forever.

- Policy + roadmap: [`bootstrap/FLOOR.md`](bootstrap/FLOOR.md)  
- Runtime ABI sketch: [`bootstrap/RUNTIME_ABI_v0.md`](bootstrap/RUNTIME_ABI_v0.md)  

Frontend (lex/parse/check) stays backend-neutral; lowering the floor means new emit/runtime/link packages, not rewriting the language.

## Bootstrap & release (no Cargo)

```bash
# Rebuild product from seed (no rustc)
SEED_OODAC=./oodac/oodac ./scripts/bootstrap_no_cargo.sh

# Self-host referee
./scripts/fixed_point.sh

# Rails
./scripts/p3_no_cargo_smoke.sh
./scripts/product_pure_dispatch_smoke.sh
./scripts/chs_parity.sh
./scripts/beta_cli_smoke.sh
./scripts/c_emit_smoke.sh
./scripts/ci_no_rust.sh   # B1-style: asserts no cargo on product path

# Release tarball (no cargo)
./scripts/release.sh v0.183.0-alpha
```

Builder needs **gcc + seed binary only**. See `scripts/bootstrap_no_cargo.sh`.

---

## Beta gates (honest)

| Gate | Status (this pin) |
|---|---|
| **B0** no `.rs` | PASS |
| **B1** no Cargo product build | PASS (scripts; CI matrix optional residual) |
| **B2** pure fixed-point surface | PASS (oodac) |
| **B3** ship without stage-0 Rust | PASS path (`release.sh` + seed) |
| **B4** honesty / fail-closed residual | PASS process; **no beta tag** until public review |
| **B5** org siblings non-Rust product | PASS for product-critical siblings (std/qa/docs …); editors optional |

**Do not call this beta** until a public beta tag + install pin + notes are deliberately cut. This is **v0.183.0-alpha** with a zero-Rust product tree.

Proof / status: monorepo `PROGRESS.md`; criteria `bootstrap/BETA.md`; latest ship notes `RELEASE_NOTES_v0.183.0-alpha.md`.

---

## Related

`spec`, `qa`, `std` (`.oo` only), `openOODA.github.io`, optional `tree-sitter` / `vscode` (editor support — not compiler critical path).

Ship notes: [GitHub Releases](https://github.com/openOODA/ooda/releases).
