# Build-out backlog (guided by DESIGN, not limited by it)

**Purpose:** Living list of **product gaps** so rotation loops **work on real codebase growth** — not only purity rails or a frozen beta surface.

**Rules:**
- **`DESIGN.md` guides** — prefer work that advances its pillars (caps, self-test, AI-native, systems/native).
- **DESIGN does not forbid** better product ideas that **align** with those pillars (clearer CLI, std, backends, ship path, agent ergonomics).
- **Propose → implement with tests → document** if we add something DESIGN never named.
- **Unfinished = fail-closed**, not silent.
- **Beta tag = owner only** (`BETA.md`). This file is **build-out**, not auto-beta.
- **No Rust product host.** `.oo` + thin C (or later FLOOR backends).
- **≤256 lines / owned file**; E-M + entropy \(S\); tests as immune system.

**How the loop uses this:**  
Observe → read PROGRESS + **this file** → pick highest-value open item (or split it) → Act with pass+fail fixtures → Lock rails → Ship → mark item or note residual.

---

## Priority bands (loop Decide order within build-out)

After fixing **red rails / purity regressions / honesty lies**, prefer:

| Band | Meaning |
|------|---------|
| **P0** | Development loop broken or DESIGN pillar completely absent on pure path |
| **P1** | Core systems loop: check → test → build → run for real programs |
| **P2** | AI-agent loop + token efficiency |
| **P3** | Platform depth: std, ship, multi-backend, polish |
| **P4** | Stretch / later |

Reorder freely; owner steers. Agents should **not** ignore P0–P2 forever to polish entropy only.

---

## Open items (start from current pure product)

### P0 — Keep the runway green
> **Standing rails** — not one-shot ships. When GHA is green, treat as *standing — verified by CI* (re-check on every pin / red PR). Local: `scripts/ci_product.sh`. Remote: `.github/workflows/product.yml`.

- [x] Pure rails stay green **locally**: `fixed_point`, `chs_parity`, `c_emit_smoke`, product smokes, `bootstrap_no_cargo` / `ci_product` — *standing local rail; remote GHA residual per `GHA_PRODUCT.md`*
- [x] `RS_COUNT=0`; no Cargo product path — *standing local* (shadow cargo in `ci_product`; GHA when seed resolvable)
- [x] Line lock \(O=0\) — *standing rail* (`scripts/check_file_lines.sh`); O=0 on main (re-check every pin)

### P1 — Core systems development loop
- [x] **Contracts on native path** — Backend-C lowers **simple `requires IDENT OP lit|ident`** + **simple `ensures result OP lit|ident`** at runtime (`c_emit_contract.oo`); structural skip still correct
  - **M51 multi-clause simple AND In** — multiple simple `requires`/`ensures` on one fn lower as sequential runtime checks (ensures cap 8)
  - **M159/M162/M165 path A** — simple `&&` / `||` and simple arith+compare (`x + 1 > 0`, `(a>0||b>0)&&c>=0`) runtime emit (not SMT; **no quantifiers / old-state**)
  - Pass: `fixtures/requires_simple.oo` / `ensures_simple.oo` / `multi_clause_pass.oo` / `complex_contract_pass.oo` / `contract_arith_pass.oo` + `bootstrap/corpus/emit-c/pass/fn_contracts_add.oo`
  - Fail: `fixtures/requires_fail.oo` / `ensures_fail.oo` / `multi_clause_{req,ens}_fail.oo` / `contract_arith_*_fail.oo`; `contract_no_brace.oo`
  - Smoke: `scripts/contracts_native_smoke.sh` (+ multi_clause / and / arith smokes) + `scripts/problem_hunt_smoke.sh`
  - Residual honesty: `CONTRACTS_COMPLEX.md` (no full SMT / quantifiers / old-state)
- [x] **Real `ooda test`** — run `verify` blocks (not only typecheck)
  - Pure path: check → lower `assert_eq!`/`assert_ne!`/`assert!` in `verify` → emit-c+gcc harness (no Python / pure_build)
  - Scripts: `scripts/ooda_test_verify.sh` + `ooda_verify_pure.sh`; CLI `ooda test`; smoke `verify_pure_smoke.sh`
  - Fixtures: `fixtures/verify_pass.oo` / `verify_fail.oo`; product smoke rails
  - Residual: verify supports `assert_eq!`/`assert_ne!`/`assert!`/`let` only; complex contracts residual
  - Legacy: `ooda_test_harness.py` retained but off critical path
- [x] **`ooda test --fuzz` pure Int/Bool/String/List domains** — CLI un-gated; **`ooda_fuzz_pure.sh`** (no Python on `--fuzz` path); fixtures `fuzz_{int,bool,string,list}_{domain,fail}.oo`
  - Residual: multi-param pure fuzzer **not** shipped; other domains fail closed (`FUZZ_DEFER.md`)
  - Do **not** claim full multi-type pure-native fuzzer
- [x] **Caps completeness on claimed path** — Fs/Sys/Env/(Net) matrix: lower or fail-closed consistently; expand sealed C allowlist with fixtures
  - Matrix: `bootstrap/CAPS_MATRIX.md` (keep matrix honest vs `AUDIT_RESIDUAL` — `fetch` may lower)
  - Check seal: `is_sealed_{fs,sys,env,net}` incl. `env_get`, `path_exists`, `file_size`
  - Emit: real lower for read/write/path/size/env/sys; **`fetch` → `oo_fetch`** (AUDIT R9); other net names residual
  - Fixtures: `bootstrap/corpus/check/{pass,fail}/` per class; rail `scripts/caps_matrix_smoke.sh`
  - Runtime seal: magic tokens + `oo_cap_require` on FS/Sys/Env (`STATIC_CAPS.md`); forge deny in `caps_matrix_smoke`
  - Residual: other net ops residual; dynamic callees residual; not cryptographic object-caps
- [x] **Richer `ooda run`** — **permanent pure native build+exec** (no host interpreter return)
  - Documented in README + help; clearer errors (missing file / build fail / no exe)
  - Residual: no JIT/interpret path on product surface
- [x] **Import load honesty** — real multi-file load in oodac (reduce bash-concat residual) with cycle/missing fail fixtures
  - In-tree: `oodac/load_import.oo` expands imports for check|tokens|ast; cycle/missing → `ERR\timport\t…`
  - Residual: emit-c multi-module still `EMIT_NO_CONCAT=1` per-file; optional `EMIT_CONCAT=1` → hardened `scripts/oodac_concat.sh`
  - pure_build: nested import collect + cycle/missing fail-closed
  - Fixtures: `bootstrap/corpus/import/{pass,fail}/`; smoke: `scripts/import_load_smoke.sh`
- [x] **Typecheck depth** — close gaps vs SPEC/CHS needs for real programs (methods, structs, refinements, …) with corpus
  - Refine: `return -N` vs `Int[lo..hi]` fail-closed; List/Result ann bind (base name + bracket skip)
  - Methods: `.to_string` on primitives; `.is_ok`/`.is_err` need Result; builtin list_* arity
  - Corpus: refine_ret_*, to_string_int_ok, result_is_ok, list_len_method, list_get_arity, is_ok_on_int
  - Residual: full generic List[T]/Result[T,E] element typing; must-use on call-returning Result

### P2 — AI-native / agent loop (DESIGN §3 + aligned extras)
- [x] **`--json-errors`** (or successor) on pure check path
  - `oodac check --json-errors` / `-json` → JSON array `{code,line,col,msg,path}`
  - Product `ooda check --json-errors` forwards to oodac (not residual)
  - Codes: `bootstrap/DIAG_CODES.md`; smoke: `scripts/json_errors_smoke.sh`
  - Residual: no suggested_fix / timings (not host AiDiagnostic)
- [x] **`ooda outline`** — token-cheap API summary
  - Pure path: `oodac outline` via product CLI (M1); not Python helper
  - One line per `pub fn` (params, ret, `caps=…`); fail-closed unreadable
  - Format: `bootstrap/OUTLINE_REFLECT.md`; rail `scripts/outline_reflect_smoke.sh`
  - Residual: not full typed AST outline; no import graph lines yet
- [x] **`ooda reflect`** — symbol/contract/cap metadata
  - NDJSON: fn (requires/ensures/caps) + verify names; optional symbol filter
  - Pure `oodac reflect` + smoke; never executes user code
- [x] **`ooda patch`** — surgical `replace_fn` for agents (SAFE)
  - CLI: `ooda patch <file.oo> --replace-fn <name> --with <body_file> [--check]`
  - Or JSON stdin `{"op":"replace_fn","name":…,"body":…}` only (unknown op fail-closed)
  - Security: no shell-eval of body; reject `..`; relative under cwd; atomic write
  - Engine: `scripts/ooda_patch.py` + `scripts/ooda_patch.sh`; rails: `scripts/patch_smoke.sh`
  - Fixtures: `fixtures/patch_add.oo` + `patch_add_body.txt`
  - Residual: no line-range op yet; no AST node_id path
- [x] **Stable diagnostic codes** for agent routing (aligned with DESIGN intent)
  - `E_CAP` | `E_TC` | `E_PARSE` | `E_LEX` | `E_CHECK` | `E_LOAD` | … — `bootstrap/DIAG_CODES.md`
- [x] Optional: agent-oriented “fix suggestion” without reintroducing host bloat
  - `fix_hint` field on `--json-errors` objects (`diag_fix_hint`); no AST rewrite / no host bloat
  - Residual: not surgical AST patches (see `ooda patch` for edits)

### P3 — Platform & ship
- [x] **std growth** — modules real programs need (aligned with caps)
  - Pure path: `std/result.oo`, `std/str.oo`, `std/option.oo` (+ `std/README.md`)
  - Fixtures: `fixtures/std_{result,str,option}_main.oo` (pure multi-build); library `oodac check`
  - Residual: no ambient FS/net; `Option[T]` sum emit not pure-lowered (Result encoding); multi-file check residual; generic Result beyond String residual; org-sibling json/crypto/fs/net not pure floor
- [x] **Install / pin dress rehearsal** — `scripts/install_dress_rehearsal.sh` validates release layout offline (tarball / staged tree / working-tree stage); residual: full XDG `install.oo` network fetch not exercised offline
- [x] **Remote CI product rail** — `.github/workflows/product.yml` runs `ci_product` with cargo/rustc shadowed; seed = `bootstrap/seed/oodac` or pinned GitHub Release tarball+sha256 (no rustup/cargo install)
- [x] **Release notes + pin lock** every ship — `bootstrap/RELEASE_CHECKLIST.md` + `release.sh` sha256 sidecar + notes reminder; habit not auto-beta
- [x] **FLOOR F3** — second backend MVP when owner prioritizes freedom over features
  - Prep: `bootstrap/BACKEND_F3_PREP.md` + `FLOOR.md` + `RUNTIME_ABI_v0.md`
  - Product: only `--backend c` allowlisted; non-c fail-closed (scaffold door)
  - Residual: no second emit/runtime/link package yet (owner prioritizes when ready)
- [x] Shrink emit preamble **host residual** decls (`oo_host_*`) on pure path when safe
  - Pure preamble no longer declares `ooda_host_*` / `oo_host_*` / `oo_chs_build`
  - `runtime/chs_rt_host.c` kept for optional `OODA_WITH_HOST_FFI` only (not pure link)
  - Residual: optional host FFI path; programs that need host dumps must opt into FFI

### P4 — Stretch (DESIGN or aligned)
- [x] **Deterministic pure multi input fingerprint** (M20 / PM 4.3.2 partial depth)
  - `scripts/oodac_pure_build.sh` prints `pure_build: input_fp=<sha256 hex>` over module `relpath\0`+contents (order = MODS)
  - Same tree → same fingerprint on two pure multi runs; optional `PURE_BUILD_FP_OUT`
  - Smoke: `scripts/pure_build_fp_smoke.sh` (in `ci_product` after residual/seed rails)
  - Doc: `bootstrap/PURE_BUILD_FP.md`
  - Residual: **not** bit-identical product binaries / full reproducible dist (timestamps, ASLR, host toolchain)
- [x] LLVM production emit path — **M119 closed** for proven surface (CHS×4 + multi-module + product CLI O0/`--release` O3); not self-host; see `P4_DROPS.md` + `LLVM_SMOKE.md`
- [x] WASM product run floor — **not claimed**; `.wat` emit smoke only (M4 partial); `P4_DROPS.md`
- [x] Packaging/registry — **fail-closed residual** (no product registry; ship = tarball+pin only); `P4_DROPS.md`
- [x] Concurrency / async — **not in product**; DESIGN-aligned future only; `P4_DROPS.md`

---

## Done criteria for an item

An item may be checked only when:

1. Pure product path implements it (or deliberate fail-closed with owner-visible residual), and  
2. Pass **and** fail fixtures (or equivalent rail) exist, and  
3. \(O\) not worsened without a split, and  
4. PROGRESS notes the ship.

---

## Relationship to BETA.md

| File | Use |
|------|-----|
| **BUILD_OUT.md (this)** | What to **build next** for a better product |
| **BETA.md** | What must be true **if/when owner tags beta** (purity + optional frozen In list) |

Growing BUILD_OUT does **not** require updating BETA.  
When a feature becomes “must ship in first beta,” **owner** may promote it into BETA B.1.

---

*Recompute and reorder every few pins. Delete done noise; don’t let this become a vanity checklist.*
