# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.24.0-alpha`**

OODA (Observe, Orient, Decide, Act) — capability-secure, self-testing, AI-native systems language.

> **DESIGN.md** is the north star (unchanged by alpha releases).  
> Userland is **`.oo`**. **CHS self-host frontend** is green (`oodac` native + fixed-point referee).  
> Full SPEC product self-host is **not** claimed.

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
ooda --version   # 0.24.0-alpha
```

```ooda
import std::crypto;   // needs OODA_STD pointing at openooda-std
import "lib.oo";       // relative / OODA_PATH
```

---

## What's real in v0.24.0-alpha (CHS M0–M5)

| Capability | Status |
|---|---|
| `while` / `else if` / unary `!` | Real (interp + LLVM/WAT subset) |
| Option / Result / must-use / let mut | Real |
| Caps + sealed effects | Real (static + runtime) |
| **CHS:** `List`, string walk, structs, real FS, argv | Real on interpreter |
| **CHS C backend** (`ooda build --target c`) | Real — gcc + `runtime/chs_rt.c` (no clang required) |
| **Canonical dumps** `ooda dump tokens\|ast\|check` | Real |
| **oodac** (`oodac/main.oo`) lex/parse/check/smoke-build | Real (interp + native) |
| **Parity / fixed-point** | `scripts/chs_parity.sh`, `scripts/fixed_point.sh` |
| LLVM Int/Bool/Float + while | Real (clang to link when present) |

## Not implemented (fail non-zero)

LSP, pkg install, migrate, replay, full WASM product, PyTorch, `for` sugar, full SPEC self-host (only **CHS frontend** fixed-point is claimed).

See `bootstrap/CHS.md`.

---

Related: `spec`, `qa`, `std` (.oo), `openOODA.github.io`.
