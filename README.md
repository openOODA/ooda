# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.19.0-alpha`**

OODA (Observe, Orient, Decide, Act) is a systems-oriented, guard-rail-first
language for capability security, self-verification, and AI co-authoring.

> **DESIGN.md is the architectural north star (do not rewrite it to match alpha gaps).**  
> This README states what the binary does **today**.

**Language rule of thumb:** *userland, std, tests, and examples are `.oo`.*  
The **compiler bootstrap** remains Rust until self-hosting is real (normal for new languages).

---

## Quick Start

```bash
git clone https://github.com/openOODA/ooda.git
cd ooda
cargo build --release

./target/release/ooda run examples/hello.oo
./target/release/ooda check examples/hello.oo
./target/release/ooda run examples/import_lib.oo
./target/release/ooda run examples/option_match.oo
./target/release/ooda build examples/float_main.oo --emit-llvm
./target/release/ooda build --target wasm examples/int_main.oo
```

```bash
curl -fsSL https://openOODA.github.io/install.sh | sh
ooda --version   # 0.19.0-alpha
```

Import the std library (`.oo` modules):

```bash
export OODA_STD=/path/to/openooda-std
# import "crypto.oo";
```

---

## What's real in v0.19.0-alpha

| Capability | Status |
|---|---|
| Interpreter + contracts + verify | Real |
| Sealed caps + runtime gate + **cap-handle call-graph** | Real (forged cap args denied) |
| Typecheck, must-use, let mut, refinements | Real |
| Exhaustive match Result/**Option** | Real |
| **Some / None** | Real |
| **`import "file.oo"`** multi-file modules | Real (`OODA_PATH` / `OODA_STD`) |
| **`ooda check`** (parse+caps+types, no run) | Real |
| LLVM Int/Bool/**Float** IR (+ if); clang link | Real (IR-only without clang) |
| WAT Int subset (+ if, println) | Real subset |
| AI `--json-errors` with spans + fixes | Real |

## Not implemented (fail non-zero)

| Feature | Behavior |
|---|---|
| `ooda lsp` / `migrate` / `replay` | Error |
| `ooda pkg --install` | Error |
| Full WASM/WASI product | Subset WAT only |
| Python / PyTorch bridge | Honest `Err` |
| Self-hosted compiler in `.oo` | Prototypes only |

---

## Hygiene

Do not commit `*.ll`, `*.wat`, `dist/`, or release tarballs.

Related: `spec`, `qa`, `std` (`.oo`), `vscode`, `tree-sitter`, `openOODA.github.io`.
