# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.21.0-alpha`**

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
ooda --version   # 0.21.0-alpha
```

```ooda
import std::crypto;   // needs OODA_STD pointing at openooda-std
import "lib.oo";       // relative / OODA_PATH
```

---

## What's real in v0.21.0-alpha (M0 CHS host surface)

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
| **CHS M0:** real `read_file`/`write_file` under `&FsCap` | Real (interpreter; sealed) |
| **CHS M0:** `List[T]` + `list_*` | Real (interpreter; LLVM host-only until M4) |
| **CHS M0:** `chars_len` / `char_at` / `str_slice` / char class | Real (interpreter) |
| **CHS M0:** `type T = struct { … }` + field access | Real (interpreter) |
| **CHS M0:** `main(args: List[String])` via `ooda run f.oo -- …` | Real |

## Not implemented (fail non-zero)

LSP, pkg install, migrate, replay, full WASM product, PyTorch, `for` sugar, self-host / fixed-point, LLVM lower of List/String/struct (documented host-only kill date M4).

See `bootstrap/CHS.md` for the Compiler Host Subset plan.

---

Related: `spec`, `qa`, `std` (.oo), `openOODA.github.io`.
