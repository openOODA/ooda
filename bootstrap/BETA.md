# Beta exit criterion (product + purity)

**Status:** Goal (not claimed yet — still alpha).  
**Constitution:** `DESIGN.md` is the language north star and is **not** edited here.  
**This document:** criteria for a **beta tag** — purity, ship, honesty, and a **frozen beta product surface**.  
**Userland rule (all alphas + beta):** product code is **`.oo`**.

### Owner authority (non-negotiable)

**Only the project owner decides when openOODA is ready for a beta tag** (and which version string that is).

| This document does | This document does **not** |
|--------------------|----------------------------|
| Define **minimum** gates so “beta” cannot mean hollow purity or fake surface | Force a tag the day B0–B5 or Part B first go green |
| Give agents/humans a checklist to work against | Auto-declare beta from CI, agents, or PROGRESS prose |
| Allow **promoting** Out → In (grow beta surface) by deliberate table edit | Forbid staying on alpha while growing DESIGN, platform, or polish |

**Growing and improving is always allowed on alpha** (and after beta, on later releases).  
Meeting gates = **eligible** for beta consideration.  
**Shipping the tag** = owner decision only — after whatever extra growth, polish, or delay the owner wants.

Agents, rails, and collaborators may report “gates green” or “gaps remain.” They must **not** claim beta, cut a beta tag, or treat gate-green as mandatory immediate release.

---

## One-line goal

**First beta** = pure self-hosted `.oo` product toolchain **and** a **named, proven product surface** that works end-to-end; everything else **fail-closed** and documented as out-of-beta — **and** the owner chooses to tag.

Beta is **not** “DESIGN.md fully implemented.”  
Beta is **not** automatic when the checklist is green.

---

## How this doc relates to others

| Doc | Role |
|-----|------|
| **`DESIGN.md`** | What OODA *is* long-term (caps, contracts, AI-native, systems/native). Unchanged by beta. |
| **`CHS.md`** | Frozen **compiler-host subset** for self-host. Beta surface ⊇ CHS (at least enough to build the compiler). |
| **`FLOOR.md`** | Native **backend policy** (Backend-C today; other backends later). C allowed at beta. |
| **`BETA.md` (this file)** | **Definition of done for a beta tag** + in/out surface + proof. |
| **Proof log** | Record B0–B5 pass/fail in monorepo `PROGRESS.md` or latest `RELEASE_NOTES_*.md` when re-verifying. |

```text
DESIGN     →  long-term language vision
BETA       →  “may we tag beta?” (purity + frozen surface + ship)
CHS        →  bootstrap language fence
FLOOR      →  codegen/runtime floor under native builds
```

---

## Definition of done (owner may tag beta only if all true)

These gates are **necessary** for an honest beta tag. They are **not sufficient** to force a tag — see **Owner authority** above.

### Part A — Purity & ship (B0–B5)

| # | Gate | Proof |
|---|------|--------|
| **B0** | **Product purity — no `.rs` in product tree** | `find . -name '*.rs' -not -path './.git/*' -not -path './target/*' \| wc -l` → **0** |
| **B1** | **No Cargo product build** | No `Cargo.toml` / `Cargo.lock` as supported path; pure build/install (seed + gcc; script and/or CI) |
| **B2** | **Self-host fixed-point on beta surface** | Compiler in `.oo` builds itself: stage-N vs N+1 digests match for the **beta surface** (see rails below); pure path only (no host soft-pass) |
| **B3** | **Ship pure `.oo`+C path** | Release tarball / install ships `ooda` (and seed compiler as needed) from pure product pipeline only |
| **B4** | **Honesty** | Out-of-beta features **fail non-zero**; no “self-hosted” or “beta” claims that contradict residual list |
| **B5** | **Org consistency** | Product-critical siblings (`std`, `qa`, `install`, docs/site pins) do not **require** a foreign host toolchain for the beta product path |

**Allowed at beta (product purity):**

- Thin **C** runtime / link glue (`runtime/chs_rt*`) — Backend-C floor (`FLOOR.md`).
- Chapter-0 **shell** install that fetches a prebuilt binary then hands off to `install.oo`.
- Prebuilt **seed** binary (`SEED_OODAC` / `oodac` in the tarball) produced by the pure pipeline.
- Optional editor grammars (tree-sitter, etc.) that are **not** required to build or run the compiler.

**Not allowed at beta:**

- Shipping or requiring product `src/**/*.rs`, `Cargo.toml`, or `cargo build` as the supported product path (breaks B0/B1 purity).
- A beta tag while any of B0–B5 or Part B lacks captured proof.
- Claiming full DESIGN (LLVM production path, full AI suite, full net/async, …) unless those items are **in** the frozen surface below and proven.

### Part B — Frozen beta product surface

**Done for product beta** means every **In** row is green on the pure product binary; every **Out** row fails closed (non-zero) with a clear error.

#### B.1 In for beta (must work)

*Edit this table only when deliberately changing beta scope—not casually per feature PR.*  
*Pin reference for this freeze of the real alpha surface: **v0.183.0-alpha** (BUILD_OUT sweep). Still **not** a beta tag.*

| Area | In-beta requirement | Proof rails (examples) |
|------|---------------------|-------------------------|
| **CLI core** | `ooda version`, `help`, `check`, `dump tokens\|ast\|check` | `product_pure_dispatch_smoke`, `beta_cli_smoke` |
| **Build / run** | `build --target c\|chs\|native`; `run` = permanent pure **native** build+exec (no host interpreter) | `chs_parity`, product smokes |
| **Backend** | Product **self-host floor** = Backend-C; `llvm`/`wasm` emit scaffolding only (not alternate floors) | `p3_no_cargo_smoke`, emit smokes, `BACKEND_F3_PREP.md` |
| **Test** | `ooda test`: check + lower **`assert_eq!` / `assert_ne!` / `assert!`** in `verify` → pure emit-c+gcc harness (no Python) | `ooda_test_verify.sh`, `ooda_verify_pure.sh`, `verify_pure_smoke.sh`, `verify_pass.oo` / `verify_fail.oo` |
| **JSON diags** | `check --json-errors` (product + oodac): codes + code-keyed `fix_hint` (E_CAP/E_TC/E_PARSE/E_CHECK+); clean → `[]`; no AST rewrite | `json_errors_smoke`, `DIAG_CODES.md` |
| **AI agent loop** | `outline` (pub fn summary); `reflect` (NDJSON fn/verify/caps/contracts text); `patch` **`replace_fn` only** (CLI or JSON stdin; path-safe) | `outline_reflect_smoke`, `patch_smoke`, `OUTLINE_REFLECT.md` |
| **Compiler** | Pure `oodac`: tokens, ast, check, emit-c, multi-module pure build | `fixed_point`, `c_emit_smoke`, `chs_parity` |
| **Language** | At least **CHS** surface (see `CHS.md`) on check + native C path | CHS fixtures + emit pass/fail |
| **Caps (static)** | Default-deny sealed effects at check; Fs/Sys/Env/Net/Time/Rand lowered on C (process-local tokens; not crypto object-caps) | `caps_matrix_smoke`, corpus `no_cap_*` |
| **Self-host** | Seed + pure rebuild of compiler + product CLI (product purity) | `bootstrap_no_cargo`, `fixed_point` |
| **Install / pin** | Single pin string: BOOTSTRAP_PIN ↔ release ↔ site install ↔ `ooda version` | release extract smoke + install dry-run |
| **Docs** | README + release notes: In list, Out list, seed+gcc pure product path | review checklist |

#### B.2 Out of beta (must fail closed — not “missing quietly”)

| Area | Out-of-beta | Behavior |
|------|-------------|----------|
| Host Cargo / `.rs` product path | any reintroduction | Forbidden (B0/B1 product purity) |
| Full LLVM/WASM **product floor** (link/run/optimize) | emit smoke ≠ floor | do not claim production backends (`P4_DROPS.md`); `--release` fail-closed |
| `ooda test --fuzz` as full multi-type pure fuzzer | CLI un-gated; **Int/Bool/String/List pure domains shipped**; **Int arity-2/3 multi-arg In** | Only `// FUZZ_DOMAIN: int\|bool\|string\|list`; arity≥4 / multi-arg non-int fail-closed (`FUZZ_DEFER.md`) — not full multi-type multi-param fuzzer |
| complex `requires` / `ensures` | not `&&` / SMT / full contract language | simple `IDENT OP lit|ident` / `result OP lit|ident` runtime In; **multi-clause simple AND In (M51)**; complex fail-closed at emit |
| **`for` non-INT bounds** | only `INT..INT` range-for lowered | emit `for residual` (use `while`) |
| **`match` non-Result / incomplete** | Result Ok/Err stmt + match-let **In** | other shapes fail-closed (`FOR_MATCH_RESIDUAL.md`) |
| **Net ops** beyond `fetch` | `fetch` lowers + runtime exists (AUDIT R9); other net names residual | friends still `ERR … net residual` |
| **Object-caps / unforgeable tokens** | magic-int runtime seal only | not cryptographic caps ([`STATIC_CAPS.md`](STATIC_CAPS.md)) |
| Non-`c` as **self-host floor** | product rebuild is Backend-C | `llvm`/`wasm` backends are emit scaffolding, not alternate self-host floors |
| Full SPEC beyond CHS + explicit B.1 promotions | post-beta | fail-closed or not advertised |
| `patch` line-range / AST node_id | residual | fail-closed / not shipped (`replace_fn` only is In) |
| **`#[MaxCycles]` / OS MaxCycles** | path A/B In: `// MAX_CYCLES: N` Backend-C `while` + range-`for` fuel (M48/M54; zero-N fail-closed) | **not** OS cgroup / recursion / attr lower — `MAX_CYCLES.md` |
| **`#[Secret]` / `// SECRET:`** | path A/B In: println bare IDENT refuse + direct assign-prop + check dual (M52–M55) | **not** interproc / NetCap suite / full taint / `#[Secret]` attr — `SECRET_TAINT.md` |

Promoting an Out item to In requires: implementation + pass/fail rails + this table edit + B4 still true for what remains Out.

#### B.3 How we know Part B is done

```text
for each row in B.1:
  pass fixtures green on pure bin/ooda + oodac
  fail fixtures non-zero where meaningful
for each row in B.2:
  documented + non-zero (or explicitly “not shipped”)
B0–B5 green with proof log (PROGRESS.md or latest RELEASE_NOTES_*.md)
public notes match In/Out tables
```

**No row in B.1 ⇒ not a beta requirement.**  
**If it should block beta, put it in B.1.** That is how “done” stays knowable.

---

## What beta is / is not

| Beta **is** | Beta **is not** |
|-------------|-----------------|
| Pure `.oo` product + self-host + ship | Full DESIGN complete |
| Frozen In surface proven | “Feature complete systems platform” |
| Out surface fail-closed | Silent stubs / soft-pass |
| C floor allowed (Backend-C) | Claim of zero low-level floor |
| Seed binary allowed | “No trusted binary ever” |

---

## Current status (recompute each Observe — not a beta claim)

**Observe pin: v0.183.0-alpha** (BUILD_OUT 8-agent sweep; B.1 table edited to match real alpha surface).  
**Public beta tag: not claimed. Owner-only.**

| Piece | Status @ v0.183.0-alpha |
|-------|-------------------------|
| B0 RS=0, no Cargo.toml | **PASS** (`RS_COUNT=0`; no `src/`, no `Cargo.toml`) |
| B1 no-Cargo build scripts | **PASS local** (`ci_product` / cargo shadow); remote GHA residual — private-asset / seed download historically red (`GHA_PRODUCT.md`) |
| B2 pure fixed_point | **PASS** (`fixed_point.sh`; s1≡s2; no OK_HOST; default **`PURE_NO_ARC=0`**; runtime release leak-safe — see `ARC_M2_RESIDUAL.md`) |
| B3 release/install path | **PASS path** (`release.sh` packs pure bins + runtime C); dress rehearsal offline OK; cold seed residual |
| B4 honesty | **PASS process** — Out rows fail-closed; alpha pin; **no beta tag cut** |
| B5 org | **PASS** product-critical siblings no Rust; editors optional |
| Part B.1 In surface | **Table promoted** to real alpha (check/dump/build/run/test asserts/json-errors/outline/reflect/patch replace_fn/`--backend c`); rails exist — still not “owner freezes beta forever” |
| Part B.2 Out surface | **Documented** arity≥4 / multi-arg non-int fuzz residual (Int/Bool/String/List pure + Int arity-2/3 multi-arg In), LLVM/WASM **floor** out (emit+execute smoke only), ensures incomplete, non-INT for residual, non-Result match residual, non-`fetch` net residual; FS/Sys/Env **runtime magic-token seal In** |
| Public beta tag | **Not claimed** |

Live notes: monorepo `PROGRESS.md`, latest `RELEASE_NOTES_*.md`.

---

## Rails (minimum set toward beta)

| Rail | Role |
|------|------|
| `scripts/check_file_lines.sh` | O=0 |
| `scripts/fixed_point.sh` | B2 pure self-host |
| `scripts/bootstrap_no_cargo.sh` / `ci_product.sh` | B1 path |
| `scripts/chs_parity.sh` | product ≡ pure oodac |
| `scripts/c_emit_smoke.sh` | emit pass+fail |
| `scripts/product_pure_dispatch_smoke.sh` / `beta_cli_smoke.sh` | CLI surface |
| `scripts/release.sh` + extract smoke | B3 |
| Corpus under `bootstrap/corpus/` + fixtures | Part B language/caps |

---

## Roadmap (toward a beta tag — power law)

Order is **finish purity proof + freeze surface + ship dress rehearsal**, not “implement all of DESIGN.”

1. **Purity locked** — B0–B3 green with re-runnable proof (CI preferred for B1).  
2. **Freeze B.1 / B.2** — explicit edit; stop casual scope creep.  
3. **Close B.1 gaps** — only items on the In list (e.g. contracts-on-native only if listed).  
4. **Release dress rehearsal** — tarball + pin + install on clean machine (still may be `*-alpha` or `*-rc`).  
5. **Owner decides to tag beta** — optional; version scheme forward-only; notes: self-hosted, no Rust, In/Out tables, seed+gcc.  
   Until then: keep growing on **alpha** (or rc pins) as long as the owner wants.

Post-beta: grow toward DESIGN (AI suite, broader language, more backends per FLOOR) without reintroducing a Rust product host.  
Pre-beta with green gates: **still fine** — owner may keep improving without tagging.

---

## Anti-goals

- Do **not** edit `DESIGN.md` to declare beta victory.  
- Do **not** treat B0 alone as “product beta done.”  
- Do **not** delete purity gates to ship a hollow binary.  
- Do **not** advertise Out-of-beta features as working.  
- Do **not** count generated C or a checked-in seed alone as “self-host done” without fixed_point-class proof.  
- Do **not** reintroduce Cargo/Rust as the supported product path.  
- Do **not** treat green gates as an automatic beta release; **owner tags beta**.  
- Do **not** block further alpha growth because “beta criteria are already met.”

---

## Metrics (every pin until beta tag)

```text
RS_COUNT=$(find . -name '*.rs' -not -path './.git/*' -not -path './target/*' | wc -l)
OO_COUNT=$(find . -name '*.oo' -not -path './.git/*' -not -path './target/*' | wc -l)
# O=0 via scripts/check_file_lines.sh
# B0..B5 and Part B: pass/fail in PROGRESS or latest RELEASE_NOTES_*.md
```

**Alpha** may still improve surface. **Beta tag** requires this document’s Definition of done (Part A **and** Part B).

---

## Multi-LLM / agent rotations

1. Prefer work that **greens a B.1 row** or **B0–B5 proof** over random DESIGN sprawl.  
2. Prefer rails (fixed_point, parity, emit fail corpus) over untested claims.  
3. Prefer fail-closed over soft-pass.  
4. Never claim beta without Part A + Part B proof **and owner intent**.  
5. CHS grows only when self-host needs it; product surface grows only via B.1 table edit.  
6. Never cut a beta tag, rename the version to beta, or announce beta on the owner’s behalf.  
7. Prefer long alpha growth over rushing a tag when gates are merely “eligible.”

---

See also: `DESIGN.md` (vision), `CHS.md` (bootstrap subset), `FLOOR.md` (backends), `README.md` (alpha reality).
