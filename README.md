# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.13.0-alpha`**

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

Install a published binary:

```bash
curl -fsSL https://openOODA.github.io/install.sh | sh
ooda --version   # 0.13.0-alpha
```

---

## What's real in v0.13.0-alpha

| Capability | Status |
|---|---|
| Lexer / parser / AST + spans | Real |
| Interpreter (`ooda run`) | Real |
| `requires` / `ensures` / `verify` | Real at runtime |
| Sealed caps + runtime gate | Real |
| Static type checker | Real |
| **Must-use `Result` / `Option`** | Real (discarded values error) |
| **`let` immutability + `let mut` assign** | Real |
| Integer LLVM IR (+ basic `if`) | Real; links via clang/cc when available |
| Integer WAT + `println(Int)` | Real subset; fails closed outside subset |
| SHA-256 / HMAC / JSON | Real |
| `--json-errors` with line:col | Real |
| outline / reflect / context / patch | Real (limited) |

## Not implemented (fail non-zero)

| Feature | Behavior |
|---|---|
| `ooda lsp` / `migrate` / `replay` | Error |
| `ooda pkg --install` | Error (`--init` only) |
| Full WASM/WASI product | Subset WAT only |
| Python / PyTorch bridge | Honest `Err` |

---

## Repo hygiene

Do not commit `*.ll`, `*.wat`, `*.wasm`, `dist/`, or release tarballs. Use `scripts/release.sh` + GitHub Releases.

Related: `spec`, `qa`, `std`, `vscode`, `tree-sitter`, `openOODA.github.io`.
