# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.18.0-alpha`**

OODA (Observe, Orient, Decide, Act) is a systems-oriented, guard-rail-first
language for capability security, self-verification, and AI co-authoring.
This repository is the compiler / interpreter / CLI toolchain.

> **DESIGN.md is the architectural north star and is not rewritten to match alpha gaps.**
> This README states what the binary actually does today.

---

## Quick Start

```bash
git clone https://github.com/openOODA/ooda.git
cd ooda
cargo build --release

./target/release/ooda run examples/hello.oo
./target/release/ooda test examples/math_contract.oo
./target/release/ooda build examples/int_main.oo --emit-llvm
./target/release/ooda build --target wasm examples/int_main.oo
```

```bash
curl -fsSL https://openOODA.github.io/install.sh | sh
ooda --version   # 0.18.0-alpha
```

---

## What's real in v0.18.0-alpha

| Capability | Status |
|---|---|
| Lexer / parser / AST + spans | Real |
| Interpreter (`ooda run`) | Real |
| `requires` / `ensures` / `verify` | Real; **call-site line:col** on contract failures |
| Sealed caps + runtime gate | Real |
| Static type checker | Real |
| Must-use Result/Option | Real |
| `let` / `let mut` assignment | Real |
| **Exhaustive `match` on Result/Option** | Real (static) |
| Integer LLVM IR (+ `if`) | Real; **native link only via clang** (never gcc on `.ll`) |
| Integer WAT + `println(Int)` + `if` | Real subset |
| `--json-errors` actionable fixes | Real (must-use / match / mut / caps) |
| outline / reflect / context / patch | Real (limited) |

## Not implemented (fail non-zero)

| Feature | Behavior |
|---|---|
| `ooda lsp` / `migrate` / `replay` | Error |
| `ooda pkg --install` | Error |
| Full WASM/WASI | Subset WAT only |
| Python / PyTorch | Honest `Err` |
| Native binary without clang | IR-only (install clang to link) |

---

## Hygiene

Do not commit `*.ll`, `*.wat`, `dist/`, release tarballs. Use `scripts/release.sh` + GitHub Releases.

Related: `spec`, `qa`, `std`, `vscode`, `tree-sitter`, `openOODA.github.io`.
