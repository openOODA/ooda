# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.11.0-alpha`**

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
```

Or install a published Linux binary:

```bash
curl -fsSL https://openOODA.github.io/install.sh | sh
# pins v0.11.0-alpha from GitHub Releases by default
```

---

## What's real in v0.11.0-alpha

| Capability | Status |
|---|---|
| Lexer / parser / AST | Real (tokens carry **line:col**) |
| Interpreter (`ooda run`) | Real |
| `requires` / `ensures` / `verify` | Real at runtime |
| Sealed capability effects + runtime gate | Real |
| Static type checker | Real |
| Integer-subset LLVM IR + validation | Real |
| SHA-256 / HMAC / JSON (`serde_json`) | Real |
| OS threads for async spawn/join | Real (minimal) |
| `--json-errors` with spans | Real |
| `ooda outline` / `reflect` / `context` | Real (context from outline/reflect) |
| `ooda patch` | Real body replace (validated parse) |
| Division-by-zero | Language error (no host panic) |

## Not implemented (commands fail non-zero)

| Command | Behavior |
|---|---|
| `ooda lsp` | Error — no JSON-RPC LSP |
| `ooda pkg --install` | Error — no downloader ( `--init` writes empty manifest only) |
| `ooda migrate` | Error — no codemods |
| `ooda replay` | Error — no tracer |
| `ooda build --target wasm` | Error — no WASM emission |
| Python / PyTorch bridge | Returns `Err("not implemented…")` |

---

## Project layout

* `src/` — compiler, interpreter, CLI
* `examples/` — reference programs
* `ooda.ebnf` — grammar snapshot (authoritative copy also in `openOODA/spec`)

Related org repos: `spec`, `qa`, `std`, `vscode`, `tree-sitter`, `openOODA.github.io`.
