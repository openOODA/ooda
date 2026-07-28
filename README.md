# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.12.1-alpha`**

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
./target/release/ooda bench examples/unauthorized_io.oo
```

Or install a published Linux binary:

```bash
# https://github.com/openOODA/ooda/releases/tag/v0.12.1-alpha
curl -fsSL https://openOODA.github.io/install.sh | sh
# or: tar xzf ooda-v0.12.1-alpha-linux-x86_64.tar.gz && export PATH=...
ooda --version   # 0.12.1-alpha
```

---

## What's real in v0.12.1-alpha

| Capability | Status |
|---|---|
| Lexer / parser / AST | Real (tokens carry **line:col**) |
| Interpreter (`ooda run`) | Real |
| `requires` / `ensures` / `verify` | Real at runtime |
| Sealed capability effects + static check | Real |
| Runtime capability gate | Real |
| Static type checker | Real |
| Integer-subset LLVM IR + validation | Real (incl. basic `if` lowering) |
| Integer-subset WASM text (`.wat`) | Partial — subset only; rejects unsupported ops non-zero |
| SHA-256 / HMAC / JSON | Real (`sha2` / `hmac` / `serde_json`) |
| Async spawn/join | Minimal (`std::thread`) |
| `--json-errors` with spans | Real |
| `outline` / `reflect` / `context` / `patch` | Real (limited) |
| `ooda fmt` | Structured printer |
| `ooda pkg --init` | Writes empty `ooda.json` |
| `ooda bench` | Per-proof verdicts (no synthetic all-green) |

## Not implemented (fail non-zero or honest Err)

| Command / feature | Behavior |
|---|---|
| `ooda lsp` | Error — no JSON-RPC LSP |
| `ooda pkg --install` | Error — no downloader |
| `ooda migrate` / `ooda replay` | Error — no codemods / tracer |
| Full WASM/WASI binaries | Not full product; subset WAT only |
| Python / PyTorch bridge | `Err("not implemented…")` |

---

## Repo hygiene

- **Do not commit** build outputs: `*.ll`, `*.wat`, `*.wasm`, `dist/`, release `*.tar.gz`.
- Publish binaries only via **GitHub Releases** (`scripts/release.sh`).
- `examples/prototypes/` holds illustrative self-host sketches — not production toolchain inputs.

---

## Project layout

* `src/` — compiler, interpreter, CLI (Rust)
* `examples/` — reference `.oo` programs
* `scripts/release.sh` — build + upload release asset (artifacts stay gitignored)
* `ooda.ebnf` — grammar snapshot (also in `openOODA/spec`)

Related org repos: `spec`, `qa`, `std`, `vscode`, `tree-sitter`, `openOODA.github.io`.
