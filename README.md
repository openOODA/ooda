# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA` — **Version `v0.65.0-alpha`**

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

./target/release/ooda run fixtures/hello.oo
./target/release/ooda run fixtures/while_count.oo
./target/release/ooda check fixtures/hello.oo
./target/release/ooda build fixtures/while_count.oo --emit-llvm
./target/release/ooda build --target wasm fixtures/while_count.oo
```

`fixtures/` is harness input only (tests, parity, fixed-point) — not a tutorial pack.  
Historical demos remain in git history under the old `examples/` path.

```bash
curl -fsSL https://openOODA.github.io/install.sh | sh
ooda --version   # 0.65.0-alpha
```

```ooda
import std::crypto;   // needs OODA_STD pointing at openooda-std
import "lib.oo";       // relative / OODA_PATH
```

---

## What's real in v0.65.0-alpha (CHS M0–M5)

| Capability | Status |
|---|---|
| `while` / `else if` / unary `!` | Real (interp + C + LLVM/WAT; WASM while polarity + i32→i64 compare extend) |
| Option / Result / must-use / let mut | Real |
| Nested block scopes | Real: `let` inside if/while does **not** leak; match-arm pattern shadows restore; outer `let mut` assign in match+if persists |
| Type aliases | Real: unify for arith/return; `Int[lo..hi]` on params **and** let/return (const TC + runtime) |
| Caps + sealed effects | Static **and runtime** object-cap: free sealed ops need live handle Value; ambient-only denied |
| `?` try-operator | Real: unwraps Result; early-return on Err; only in Result-returning fns; build refuses outside interp |
| Bool match | Real: `true`/`false` patterns + exhaustiveness |
| `.contains` | Real on String (interp + CHS C) |
| Field assign | Real: `p.x = v` on `let mut` structs (interp + CHS C) |
| list_get const OOB | Real: const negative / known list_new+push length chains fail at typecheck |
| Nested/tail return refine | Real: const `return` inside if/while + tail expr enforces `Int[lo..hi]` / aliases |
| `Int[lo..hi]` refinement | Real on let/assign/return/params **including via type aliases**; const typecheck + runtime |
| Types fail-closed | Missing returns fail; match arms unify; same-type arith/eq; if-value needs else; assert_eq types |
| Net GET (`fetch` / `http_get` / `.get`) | Real HTTPS via curl under threaded `&NetCap` |
| AI diagnostics (`--json-errors`) | Real JSON + measured timings; patch codemods including `refinement_bounds` |
| Measured `ooda em` / `em --json` / `bench --em` | Real clocks only (W, µs, V); JSON EmReport for agents — no fake Boyd Ps |
| String methods | Real on interpreter + CHS C (`.char_at` / `.str_slice`); LLVM subset refuses strings fail-closed |
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

Ship notes: [GitHub Releases](https://github.com/openOODA/ooda/releases) (not root RELEASE_NOTES files; history recoverable via git).

Related: `spec`, `qa`, `std` (.oo), `openOODA.github.io`.
