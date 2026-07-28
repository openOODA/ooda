# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.9.0-alpha`**

OODA (Observe, Orient, Decide, Act) is a modern, systems-oriented, guard-rail-first
programming language designed for high reliability, capability security,
zero-day defense, self-verification, and rapid AI co-authoring ("vibe coding").

---

## ⚡ Quick Start

```bash
# Clone the compiler repo
git clone https://github.com/openOODA/ooda.git
cd ooda

# Build the compiler toolchain
cargo build --release

# Run a .oo file using the interpreter
./target/release/ooda run examples/hello.oo

# Run contracts and inline verify tests
./target/release/ooda test examples/math_contract.oo

# Generate LLVM IR for an Int-only program (string programs are rejected with a
# clear error and a pointer to use `ooda run` instead)
./target/release/ooda build examples/math_contract.oo
```

---

## What's Real in v0.9.0-alpha

| Capability | Status | Notes |
|---|---|---|
| Lexer / Parser / AST | Real | Hand-written, ~3,800 lines of Rust |
| Tree-walking interpreter | Real | `ooda run` and `ooda test` execute correctly |
| `requires` / `ensures` contracts | Real | Evaluated at runtime |
| `verify { ... }` blocks | Real | Run during `ooda test` |
| Refinement types (`where 1..=N`) | Parsed | Bounds enforced via `requires` clauses |
| Static capability check | Real | Sealed effect table, no substring matching |
| **Runtime capability check** | **Real** | Default-deny enforced in the interpreter itself |
| `crypto_sha256_internal` | Real | Uses the `sha2` crate; verified against RFC vectors |
| `crypto_hmac_sha256_internal` | Real | Uses the `hmac` crate |
| `json_parse_internal` / `json_stringify_internal` | Real | Uses `serde_json` |
| `async_spawn_internal` / `async_join_internal` | Real | Spawns `std::thread` handles, joins them |
| `python_embed_internal` | Honest stub | Returns `Err("not implemented in this alpha")` |
| LLVM IR backend (Int subset) | Real | Validates via `llvm-as` when available |
| LLVM IR backend (String/Float/...) | Rejected with clear error | Use `ooda run` for full language surface |
| Static type checker (`bool + int` etc.) | Real | Wired into `ooda run` |
| Fuzz harness | Real | Cartesian product over parameter boundaries |

## What's NOT Real (Honest Gaps)

* **WASM target** is not implemented. The CLI rejects `--target wasm`.
* **Python bridge** is not implemented; calling it returns an honest Err.
* **LSP server**, **outline**, **patch**, **context**, **replay**, **migrate**,
  and **pkg** subcommands exist as scaffolding and print placeholder output.
* **No garbage collector**: the interpreter uses `Box`/`HashMap` and there are
  no region arenas. The "0ms GC pause" claim from earlier alphas is not
  applicable because there is no GC.

---

## 📂 Project Structure

* **`ooda.ebnf`** — Formal grammar
* **`examples/`** — Reference `.oo` programs
* **`src/`** — Compiler, interpreter, CLI toolchain
* **`docs/`** — Linked to the public docs site