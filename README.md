# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.20.0-alpha`**

OODA (Observe, Orient, Decide, Act) — capability-secure, self-testing, AI-native systems language.

> **DESIGN.md** is the north star (unchanged by alpha releases).  
> Userland is **`.oo`**; the compiler bootstrap is still **Rust** until self-host.

---

## Quick Start

```bash
git clone https://github.com/openOODA/ooda.git && cd ooda
cargo build --release

./target/release/ooda run examples/hello.oo
./target/release/ooda run examples/while_count.oo
./target/release/ooda run examples/import_lib.oo
./target/release/ooda check examples/hello.oo
./target/release/ooda build examples/while_count.oo --emit-llvm
./target/release/ooda build --target wasm examples/while_count.oo
```

```bash
curl -fsSL https://openOODA.github.io/install.sh | sh
ooda --version   # 0.20.0-alpha
```

```ooda
import std::crypto;   // needs OODA_STD pointing at openooda-std
import "lib.oo";       // relative / OODA_PATH
```

---

## What's real in v0.20.0-alpha

| Capability | Status |
|---|---|
| `while` loops | Real (interp + LLVM + WAT) |
| `else if` chains | Real |
| Unary `!` (and `-`) | Real |
| `.is_err()` / `.is_ok()` | Real |
| `import "file.oo"` / `import std::name` | Real |
| Option / Result / must-use / let mut | Real |
| `ooda check` | Real |
| Caps + call-graph handles | Real |
| LLVM Int/Bool/Float + while | Real (clang to link) |
| WAT Int subset + while/if | Real subset |

## Not implemented (fail non-zero)

LSP, pkg install, migrate, replay, full WASM product, PyTorch, arrays/`for`, self-host.

---

Related: `spec`, `qa`, `std` (.oo), `openOODA.github.io`.
