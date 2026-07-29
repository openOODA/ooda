# Beta exit criterion: zero Rust (`.rs`)

**Status:** Goal (not claimed yet — still alpha).  
**Constitution:** `DESIGN.md` unchanged; this is a *product/bootstrap* exit bar, not a redesign of the north star.  
**Userland rule (all alphas + beta):** product code is **`.oo`**. This document goes further for **beta**.

---

## Goal (one line)

**When openOODA ships its first beta, the `openOODA/ooda` product tree must contain no `.rs` files** — the stage-0 Rust host is gone; the toolchain is built and shipped without a Rust/Cargo dependency.

---

## Definition of done (beta gate)

All of the following must be true before tagging anything as **beta**:

| # | Gate | How we prove it |
|---|------|------------------|
| **B0** | **No `.rs` in tree** | `find . -name '*.rs' -not -path './.git/*' \| wc -l` → **0** (exclude only `.git`; no `src/**/*.rs`, no `tests/**/*.rs`) |
| **B1** | **No Cargo product build** | No `Cargo.toml` / `Cargo.lock` required to build or install the shipped compiler; CI builds without `rustc`/`cargo` |
| **B2** | **Self-host fixed-point (product surface)** | Compiler written in `.oo` builds itself: stage-N and stage-N+1 are bit-identical (or digest-identical) for the beta surface — stronger than CHS frontend-only fixed-point |
| **B3** | **Ship path** | `install` / release tarball ships an `ooda` binary produced **without** linking stage-0 Rust |
| **B4** | **Honesty** | Unfinished beta-out-of-scope features still **fail non-zero**; no fake “self-hosted” claims while any `.rs` remains |
| **B5** | **Org consistency** | `std`, `qa`, `install` remain `.oo` (or shell bootstrap only); no new Rust in sibling product repos |

**Allowed at beta (not Rust):**

- Thin **C** runtime / host glue (e.g. today’s spirit of `runtime/chs_rt.c`) if required for OS I/O and linking — preferred bootstrap seed over Rust.
- Chapter-0 **shell** bootstrap (`install` / `install.sh`) that only fetches a prebuilt binary and hands off to `install.oo`.
- Prebuilt **release artifacts** (binaries), as long as they were produced by the `.oo`/C pipeline.

**Not allowed at beta:**

- Shipping or requiring `src/*.rs`, `tests/*.rs`, or `cargo build` as the supported path.
- “Beta” tags while stage-0 Rust is still the real compiler.

---

## What is already true (alpha)

| Piece | Status |
|-------|--------|
| Userland / std / install story | `.oo` |
| CHS frontend (`oodac/main.oo`) fixed-point | Green (frontend only) |
| Stage-0 host (`src/**/*.rs`) | **Still required** — real product `ooda` |
| Full SPEC product self-host | **Not claimed** |

Alpha may (and will) keep growing `.oo` while Rust shrinks module-by-module. **Beta is the hard cutover.**

---

## Roadmap (depth order — power law)

Work is sequenced so each step removes real dependency on Rust, not just renames files.

### Phase R1 — Product surface in `.oo` (parity with stage-0 CLI)
Port remaining stage-0 responsibilities into `.oo` (+ C backend as needed), with golden parity against current `ooda`:

1. Lex / parse / check (extend `oodac` beyond CHS dumps)  
2. Capability checker + typechecker (full alpha surface)  
3. Interpreter **or** native path sufficient to run the compiler on itself  
4. Codegen: CHS→C (primary); LLVM/WASM only if beta still needs them — subsets must fail closed  
5. CLI: `run`, `check`, `build`, `test`, `dump`, `--json-errors`  

**Gate:** `oodac` (or successor) implements the beta CLI surface under `ooda run` / native binary, with parity scripts.

### Phase R2 — Replace stage-0 modules (delete `.rs` as you go)
For each Rust module (`eval`, `typecheck`, `codegen_*`, `capabilities`, …):

1. Implement in `.oo`  
2. Parity tests (same inputs → same diagnostics / same output digests)  
3. Switch default driver to `.oo` implementation  
4. **Delete** the corresponding `.rs`  

**Rule:** unfinished ports stay fail-closed; never dual-maintain silent stubs that claim success.

### Phase R3 — Bootstrap without Rust
1. Trusted seed: C (or last known-good native `ooda` binary) builds stage-1 from `.oo` sources  
2. stage-1 builds stage-2; digests match (product fixed-point)  
3. Release packaging uses stage-2 only  
4. Remove `Cargo.toml`, `Cargo.lock`, `src/`, Rust CI jobs  

**Gate:** B0–B5 all green on a clean machine with **no Rust toolchain installed**.

### Phase R4 — Beta tag
1. Version policy: first beta is e.g. `0.1.0-beta` / `1.0.0-beta` (exact scheme TBD; **forward only**)  
2. GitHub Release + website install pin match  
3. Public notes state: **self-hosted; no Rust in tree**  

---

## Anti-goals (keep honesty)

- Do **not** delete `.rs` early and leave a hollow beta that shells out to a hidden Rust binary.  
- Do **not** count generated C or checked-in binaries as “self-host done.”  
- Do **not** edit `DESIGN.md` to declare victory; proof is B0–B5.  
- Do **not** clear the full not-implemented list just to look beta-ready; unfinished stays fail-closed.

---

## Tracking metric (every release)

Until beta:

```text
RS_COUNT=$(find . -name '*.rs' -not -path './.git/*' -not -path './target/*' | wc -l)
OO_PRODUCT=$(find . -name '*.oo' -not -path './.git/*' | wc -l)
```

Report in release notes: **`RS_COUNT` remaining toward beta zero.**  
Alpha is allowed to have `RS_COUNT > 0`. **Beta requires `RS_COUNT = 0`.**

---

## For multi-LLM rotations

When prioritizing work toward beta:

1. Prefer ports that **delete** a Rust module over greenfield features outside the beta surface.  
2. Prefer parity + fixed-point scripts over inflated QA counts.  
3. Keep version pin discipline (Cargo only until R3; then drop Cargo).  
4. Never claim beta while B0 fails.

See also: `bootstrap/CHS.md` (CHS frontend freeze), `README.md` (alpha reality table).
