# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.12.0-alpha`**

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
# https://github.com/openOODA/ooda/releases/tag/v0.12.0-alpha
tar xzf ooda-v0.12.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.12.0-alpha-linux-x86_64:$PATH"
ooda --version   # 0.10.0-alpha
```

---

## What's real in v0.12.0-alpha

| Capability | Status |
|---|---|
| Lexer / parser / AST | Real (tokens carry **line:col**) |
| Interpreter (`ooda run`) | Real |
| `requires` / `ensures` / `verify` | Real at runtime |
| Sealed capability effects + static check | Real |
| **Runtime capability gate** | Real (rejects sealed effects even if static check is bypassed) |
| Static type checker | Real |
| Integer-subset LLVM IR + validation | Real (incl. `if` lowering) |
| SHA-256 / HMAC / JSON (`serde_json`) | Real |
| OS threads for async spawn/join | Real (`std::thread`) |
| `--json-errors` with spans | Real |
| `ooda outline` / `reflect` / `context` | Real (context from outline/reflect) |
| `ooda patch` | Real body replace (validated parse) |
| `ooda fmt` | Real structured printer (no longer overwrites with Debug AST) |
| `ooda pkg --init` | Real (writes `ooda.json`) |
| `ooda bench` | Real per-proof verdicts (no synthetic green) |
| Division-by-zero | Language error (no host panic) |

## Not implemented (commands fail non-zero)

| Command | Behavior |
|---|---|
| `ooda lsp` | Error — no JSON-RPC LSP server |
| `ooda pkg --install` | Error — no downloader (`--init` writes an empty manifest only) |
| `ooda migrate` | Error — no codemods |
| `ooda replay` | Error — no tracer |
| `ooda build --target wasm` | Error — no WASM emission |
| `python_embed_internal` | Returns `Err("not implemented…")` |

---

## Honesty notes

* The `examples/self_hosted_*.oo` files are illustrative prototypes, not a
  self-hosted compiler. They demonstrate the *shape* of what a self-hosting
  bootstrap would look like; they are not used by the production toolchain.
* There is no `.wasm` artifact in `examples/` because the WASM target is
  not implemented in this alpha. The previous `examples/hello.wasm` was a
  hardcoded WAT template and has been removed.

---

## Project layout

* `src/` — compiler, interpreter, CLI
* `examples/` — reference programs
* `ooda.ebnf` — grammar snapshot (authoritative copy also in `openOODA/spec`)

Related org repos: `spec`, `qa`, `std`, `vscode`, `tree-sitter`, `openOODA.github.io`.