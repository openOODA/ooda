# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.43.0-alpha`**

OODA (Observe, Orient, Decide, Act) — capability-secure, self-testing, AI-native systems language.

> **DESIGN.md** is the north star (unchanged by alpha releases).  
> Userland is **`.oo`**. **CHS self-host frontend** is green (`oodac` native + fixed-point referee).  
> Full SPEC product self-host is **not** claimed on alpha.  
> **Beta goal:** **zero `.rs` files** in the product tree (self-hosted; no Cargo/Rust host) — see [`bootstrap/BETA.md`](bootstrap/BETA.md).

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
ooda --version   # 0.43.0-alpha
```

```ooda
import std::crypto;   // needs OODA_STD pointing at openooda-std
import "lib.oo";       // relative / OODA_PATH
```

---

## What's real in v0.43.0-alpha (CHS M0–M5)

| Capability | Status |
|---|---|
| `while` / `else if` / unary `!` | Real (interp + LLVM/WAT subset) |
| Option / Result / must-use / let mut | Real |
| Caps + sealed effects | Static **and runtime** object-cap: free sealed ops need live handle Value; ambient-only denied |
| `Int[lo..hi]` refinement | Real on let/assign/return including nested blocks + simple const-fold (`5+6`) |
| Types fail-closed | `Unknown` is not a wildcard; `Int`≠`String`; ADT holes only inside Result/Option/List |
| Net GET (`fetch` / `http_get` / `.get`) | Real HTTPS via curl under threaded `&NetCap` |
| AI diagnostics (`--json-errors`) | Real JSON + measured timings; cap fixes as ooda-patch JSON (`applicability`) |
| Measured `ooda em` / `em --json` / `bench --em` | Real clocks only (W, µs, V); JSON EmReport for agents — no fake Boyd Ps |
| Dual engine compile | Contracts + sealed I/O **refused** on C/LLVM/WASM; IR-only link fails non-zero |
| `ooda patch` | Real body / params / return type / requires / ensures |
| `ooda migrate --edition 2026` | Partial real: exhaustive match wildcards + assigned `let`→`let mut` |
| `type T = Int where …` | Fail-closed (use `requires` / `Int[lo..hi]`) |
| **CHS:** `List`, string walk, structs, real FS, argv | Real on interpreter |
| **CHS C backend** (`ooda build --target c`) | Real — gcc + `runtime/chs_rt.c` (no clang required) |
| **Canonical dumps** `ooda dump tokens\|ast\|check` | Real |
| **oodac** (`oodac/main.oo`) lex/parse/check/smoke-build | Real (interp + native) |
| **Parity / fixed-point** | `scripts/chs_parity.sh`, `scripts/fixed_point.sh` |
| LLVM Int/Bool/Float + while | Real (clang to link when present) |

## Not implemented (fail non-zero)

LSP, pkg install, replay, full WASM product, PyTorch, `for` sugar, full SPEC self-host (only **CHS frontend** fixed-point is claimed).  
`ooda migrate` is **not** a full edition engine — only the two codemods above.

Stage-0 is still **Rust** (`src/**/*.rs`). That is intentional on alpha. **Beta exit criterion:** no `.rs` left — [`bootstrap/BETA.md`](bootstrap/BETA.md).

See `bootstrap/CHS.md` (CHS freeze) and `bootstrap/BETA.md` (zero-Rust beta gate).

---

Related: `spec`, `qa`, `std` (.oo), `openOODA.github.io`.
