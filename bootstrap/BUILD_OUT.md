# Build-out backlog (guided by DESIGN, not limited by it)

**Purpose:** Living list of **product gaps** so rotation loops **work on real codebase growth** — not only purity rails or a frozen beta surface.

**Rules:**
- **`DESIGN.md` guides** — prefer work that advances its pillars (caps, self-test, AI-native, systems/native).
- **DESIGN does not forbid** better product ideas that **align** with those pillars (clearer CLI, std, backends, ship path, agent ergonomics).
- **Propose → implement with tests → document** if we add something DESIGN never named.
- **Unfinished = fail-closed**, not silent.
- **Beta tag = owner only** (`BETA.md`). This file is **build-out**, not auto-beta.
- **No Rust product host.** `.oo` + thin C (or later FLOOR backends).
- **≤250 lines / owned file**; E-M + entropy \(S\); tests as immune system.

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
> **Standing rails** — not one-shot ships. When GHA is green, treat as *standing — verified by CI* (re-check on every pin / red PR). Local: `scripts/ci_no_rust.sh`. Remote: `.github/workflows/no_rust.yml`.

- [x] Pure rails stay green: `fixed_point`, `chs_parity`, `c_emit_smoke`, product smokes, `bootstrap_no_cargo` / `ci_no_rust` — *standing — verified by CI*
- [x] `RS_COUNT=0`; no Cargo product path — *standing — verified by CI* (shadow cargo in `ci_no_rust` + GHA)
- [ ] Line lock \(O=0\) — *standing rail* (`scripts/check_file_lines.sh`); residual O≥1 tracked in `SPLIT_PLAN.md` (e.g. `cli/main.oo`) until split

### P1 — Core systems development loop
- [x] **Contracts on native path** — Backend-C **skips** `requires`/`ensures` to LBRACE (real token skip in `c_emit_skip_contracts`); bodies emit correctly
  - Not runtime-enforced on native (honest residual); optional assert mode not landed
  - Pass: `bootstrap/corpus/emit-c/pass/fn_contracts_add.oo` + `fixtures/int_main.oo` / `hello.oo`
  - Fail: `bootstrap/corpus/emit-c/fail/contract_no_brace.oo` (mid-header garbage / missing LBRACE)
  - Smoke: `scripts/contracts_native_smoke.sh` (+ c_emit_smoke corpus)
- [x] **Real `ooda test`** — run `verify` blocks (not only typecheck)
  - Pure path: check → lower `assert_eq!` in `verify` → Backend-C harness build+run
  - Scripts: `scripts/ooda_test_verify.sh` + `ooda_test_harness.py`; CLI `ooda test`
  - Fixtures: `fixtures/verify_pass.oo` / `verify_fail.oo`; product smoke rails
  - Residual: `--fuzz` (DESIGN deferral); contracts not runtime-enforced; only `assert_eq!` in verify bodies
- [x] **`ooda test --fuzz`** — fail-closed DESIGN deferral (exit 2)
  - Message points to `bootstrap/FUZZ_DEFER.md` (when/gates for real integer-domain MVP)
  - Prefer honest residual over fake fuzz; product smoke expects non-zero
- [x] **Caps completeness on claimed path** — Fs/Sys/Env/(Net) matrix: lower or fail-closed consistently; expand sealed C allowlist with fixtures
  - Matrix: `bootstrap/CAPS_MATRIX.md`
  - Check seal: `is_sealed_{fs,sys,env,net}` incl. `env_get`, `path_exists`, `file_size`
  - Emit: real lower for read/write/path/size/env/sys; **net → ERR residual** (no silent stub)
  - Fixtures: `bootstrap/corpus/check/{pass,fail}/` per class; rail `scripts/caps_matrix_smoke.sh`
  - Residual: method-form sealed calls not scanned; net product runtime none
- [x] **Richer `ooda run`** — **permanent pure native build+exec** (no host interpreter return)
  - Documented in README + help; clearer errors (missing file / build fail / no exe)
  - Residual: no JIT/interpret path on product surface
- [ ] **Import load honesty** — real multi-file load in oodac (reduce bash-concat residual) with cycle/missing fail fixtures
- [ ] **Typecheck depth** — close gaps vs SPEC/CHS needs for real programs (methods, structs, refinements, …) with corpus

### P2 — AI-native / agent loop (DESIGN §3 + aligned extras)
- [x] **`--json-errors`** (or successor) on pure check path
  - `oodac check --json-errors` / `-json` → JSON array `{code,line,col,msg,path}`
  - Product `ooda check --json-errors` forwards to oodac (not residual)
  - Codes: `bootstrap/DIAG_CODES.md`; smoke: `scripts/json_errors_smoke.sh`
  - Residual: no suggested_fix / timings (not host AiDiagnostic)
- [x] **`ooda outline`** — token-cheap API summary
  - Parse-only: `scripts/ooda_outline_reflect.py`; CLI `ooda outline`
  - One line per `pub fn` (params, ret, `caps=…`); fail-closed unreadable
  - Format: `bootstrap/OUTLINE_REFLECT.md`; rail `scripts/outline_reflect_smoke.sh`
  - Residual: python helper (not full AST); no type/import outline lines yet
- [x] **`ooda reflect`** — symbol/contract/cap metadata
  - NDJSON: fn (requires/ensures/caps) + verify names; optional symbol filter
  - Same helper + smoke; never executes user code
- [x] **`ooda patch`** — surgical `replace_fn` for agents (SAFE)
  - CLI: `ooda patch <file.oo> --replace-fn <name> --with <body_file> [--check]`
  - Or JSON stdin `{"op":"replace_fn","name":…,"body":…}` only (unknown op fail-closed)
  - Security: no shell-eval of body; reject `..`; relative under cwd; atomic write
  - Engine: `scripts/ooda_patch.py` + `scripts/ooda_patch.sh`; rails: `scripts/patch_smoke.sh`
  - Fixtures: `fixtures/patch_add.oo` + `patch_add_body.txt`
  - Residual: no line-range op yet; no AST node_id path
- [x] **Stable diagnostic codes** for agent routing (aligned with DESIGN intent)
  - `E_CAP` | `E_TC` | `E_PARSE` | `E_LEX` | `E_CHECK` | `E_LOAD` | … — `bootstrap/DIAG_CODES.md`
- [ ] Optional: agent-oriented “fix suggestion” without reintroducing host bloat

### P3 — Platform & ship
- [ ] **std growth** — modules real programs need (aligned with caps)
- [x] **Install / pin dress rehearsal** — `scripts/install_dress_rehearsal.sh` validates release layout offline (tarball / staged tree / working-tree stage); residual: full XDG `install.oo` network fetch not exercised offline
- [x] **Remote CI no-Rust** — `.github/workflows/no_rust.yml` runs `ci_no_rust` with cargo/rustc shadowed; seed = `bootstrap/seed/oodac` or pinned GitHub Release tarball+sha256 (no rustup/cargo install)
- [x] **Release notes + pin lock** every ship — `bootstrap/RELEASE_CHECKLIST.md` + `release.sh` sha256 sidecar + notes reminder; habit not auto-beta
- [ ] **FLOOR F3** — second backend MVP when owner prioritizes freedom over features
- [ ] Shrink emit preamble **host residual** decls (`oo_host_*`) on pure path when safe

### P4 — Stretch (DESIGN or aligned)
- [x] LLVM / production optimize path — **permanent fail-closed product claim** (Backend-C only); see `bootstrap/P4_DROPS.md`
- [x] WASM product path — **permanent fail-closed** until F3 MVP; `build --target wasm` / oodac `wasm` residual; `P4_DROPS.md`
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
