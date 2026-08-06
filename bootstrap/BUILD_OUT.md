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
- [ ] Pure rails stay green: `fixed_point`, `chs_parity`, `c_emit_smoke`, product smokes, `bootstrap_no_cargo` / `ci_no_rust`
- [ ] `RS_COUNT=0`; no Cargo product path
- [ ] Line lock \(O=0\)

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
  - Residual: `--fuzz`; contracts not runtime-enforced; only `assert_eq!` in verify bodies
- [ ] **`ooda test --fuzz`** — or keep fail-closed with explicit DESIGN deferral in notes
- [ ] **Caps completeness on claimed path** — Fs/Sys/Env/(Net) matrix: lower or fail-closed consistently; expand sealed C allowlist with fixtures
- [ ] **Richer `ooda run`** — restore fast interpret *or* document native-only as permanent; if interpret, pure `.oo` or thin supported path
- [ ] **Import load honesty** — real multi-file load in oodac (reduce bash-concat residual) with cycle/missing fail fixtures
- [ ] **Typecheck depth** — close gaps vs SPEC/CHS needs for real programs (methods, structs, refinements, …) with corpus

### P2 — AI-native / agent loop (DESIGN §3 + aligned extras)
- [ ] **`--json-errors`** (or successor) on pure check path
- [ ] **`ooda outline`** — token-cheap API summary
- [ ] **`ooda reflect`** — symbol/contract/cap metadata
- [ ] **`ooda patch`** — surgical edits for agents
- [ ] **Stable diagnostic codes** for agent routing (aligned with DESIGN intent)
- [ ] Optional: agent-oriented “fix suggestion” without reintroducing host bloat

### P3 — Platform & ship
- [ ] **std growth** — modules real programs need (aligned with caps)
- [ ] **Install / pin dress rehearsal** — clean machine from release tarball
- [ ] **Remote CI no-Rust** (optional but high leverage)
- [ ] **Release notes + pin lock** every ship
- [ ] **FLOOR F3** — second backend MVP when owner prioritizes freedom over features
- [ ] Shrink emit preamble **host residual** decls (`oo_host_*`) on pure path when safe

### P4 — Stretch (DESIGN or aligned)
- [ ] LLVM / production optimize path **or** permanent honest drop from product claims
- [ ] WASM product path **or** permanent fail-closed
- [ ] Packaging/registry **only** if it stays fail-closed and cap-honest
- [ ] Concurrency / async — only with DESIGN-aligned design, not drive-by

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
