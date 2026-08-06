# Zero-Rust todo map (alpha → beta)

**Constitution:** `DESIGN.md` unchanged. Exit bar: `bootstrap/BETA.md` **B0–B5**.  
**CHS freeze:** `bootstrap/CHS.md`. **Entropy:** `TOOLS.md`. **Line lock:** `scripts/check_file_lines.sh`.  
**Handoff:** monorepo `PROGRESS.md` — report `RS_COUNT` every pin until **0**.

**Goal:** product tree has **no `.rs`**; toolchain builds/ships **without Cargo/rustc**.  
**Not the goal:** delete Rust early; hollow binary; DESIGN rewrite; beta theater.

**Baseline (recompute each Observe):**

```bash
RS_COUNT=$(find . -name '*.rs' -not -path './.git/*' -not -path './target/*' | wc -l)
OO_COUNT=$(find . -name '*.oo' -not -path './.git/*' -not -path './target/*' | wc -l)
./scripts/check_file_lines.sh   # O=0 required always
```

As of map authoring: **RS_COUNT ~194**, **OO_COUNT ~294**, pin **v0.181.0-alpha**, fixed_point **red** (host C list-pack), emit-c **thin but fixtured**.

---

## How to use this list

1. **Work top-down by phase** (P0 → P6). Do not skip P0 for “delete more .rs”.  
2. Each checkbox is a **ship**: code + pass/fail rail + O=0 + honest PROGRESS.  
3. **Power law:** prefer items that **delete** host surface or **green fixed_point** over polish.  
4. **Dual-engine** whenever check/run/build/emit semantics change.  
5. **≤250 lines / file**; splits only at functional seams (`SPLIT_PLAN` first principles).  
6. Mark done only when proof exists (script exit 0 or documented fail-closed residual).

---

# P0 — Unblock native self-build (critical path)

*Without this, stage-1 oodac never becomes the real compiler.*

## P0.1 Host / C backend correctness for oodac

- [ ] **P0.1.1** Reproduce fixed_point fail in a minimal fixture (List-returning helpers → C)  
- [ ] **P0.1.2** Fix `OoIList` vs `OoSList` (or equivalent) return typing in host `codegen_c` for scan/pack helpers  
- [ ] **P0.1.3** Golden: host `ooda build --target c` on a small multi-fn List-returning `.oo` program  
- [ ] **P0.1.4** Golden: host builds **single-file** concat of oodac **or** multi-file with imports  
- [ ] **P0.1.5** `scripts/fixed_point.sh` green end-to-end (stage-0→1, smoke, stage-1→2, digests)  
- [ ] **P0.1.6** Capture fixed_point log in release/PROGRESS; no script weaken  

## P0.2 Multi-file load (import honesty)

- [ ] **P0.2.1** Spec: import resolution rules (relative, OODA_PATH, cycle fail-closed) — document only if missing; no DESIGN change  
- [ ] **P0.2.2** oodac path-load expands `import` for **check** (not only `ooda run`)  
- [ ] **P0.2.3** oodac path-load expands `import` for **emit-c**  
- [ ] **P0.2.4** oodac path-load expands `import` for **build** (or concat pipeline is explicit debt with residual + sunset date)  
- [ ] **P0.2.5** Fail fixtures: missing import, cycle, wrong path  
- [ ] **P0.2.6** Retire or demote `oodac_concat.sh` once load is real (or keep as debug-only)  

## P0.3 emit-c coverage for oodac’s own surface

*Everything oodac source uses must lower, or oodac must be rewritten to subset.*

### Core stmt/expr

- [ ] **P0.3.1** Fail rail maintained (`bootstrap/corpus/emit-c/fail/*` + `c_emit_smoke.sh`)  
- [ ] **P0.3.2** `println` string + int + bool (pass fixtures)  
- [ ] **P0.3.3** Binary ops used by oodac (`+` string/int, cmp, logic)  
- [ ] **P0.3.4** `if` / `else if` / `else` / `while` (done partial — lock full parity)  
- [ ] **P0.3.5** `let` / `let mut` / assign  
- [ ] **P0.3.6** `return` / early return in branches  
- [ ] **P0.3.7** Nested blocks / scopes  

### Types & data

- [ ] **P0.3.8** `List[Int]` / `List[String]` (new/push/get/len) — **blocking for token packs**  
- [ ] **P0.3.9** `String` ops used by oodac (concat, slice, char_at, contains, …)  
- [ ] **P0.3.10** `Result` / `Option` match + `Ok`/`Err`/`Some`/`None`  
- [ ] **P0.3.11** Struct field get/set if oodac needs (or avoid in oodac)  
- [ ] **P0.3.12** Type aliases as used in oodac  

### Calls & control

- [ ] **P0.3.13** Multi-arg calls / user fns (not only main)  
- [ ] **P0.3.14** Method-style calls oodac uses  
- [ ] **P0.3.15** `match` (Option/Result minimum)  
- [ ] **P0.3.16** `break` / `continue` if present in oodac  

### Caps & host effects (CHS)

- [ ] **P0.3.17** `&FsCap` / `read_file` / `write_file` lowering  
- [ ] **P0.3.18** `&SysCap` / `sys_exec` / `process_exit`  
- [ ] **P0.3.19** `&EnvCap` / `env_get` if needed  
- [ ] **P0.3.20** Fail-closed: unsupported construct → `ERR\tc_emit\t…` + non-zero  

### Emit engineering

- [ ] **P0.3.21** Keep `c_emit*.oo` ≤250; split at seams (`stmt` / `expr` / `call` / `preamble`)  
- [ ] **P0.3.22** `c_emit_smoke` expands as fixtures grow; optional gcc+run only for pass  
- [ ] **P0.3.23** Dual-engine: host `build --target c` vs oodac `emit-c`+gcc on shared corpus where both claim support  

## P0.4 Stage-1 oodac as a binary people can run

- [ ] **P0.4.1** Document build recipe: stage-0 → `oodac` binary  
- [ ] **P0.4.2** `oodac tokens|ast|check` on corpus without `ooda run`  
- [ ] **P0.4.3** `oodac build` pure-CHS smoke **without libooda** when no host FFI  
- [ ] **P0.4.4** Parity script: stage-0 dump vs stage-1 oodac dump (tokens/check digests)  
- [ ] **P0.4.5** CI job (or local gate): fixed_point + c_emit_smoke + chs_parity  

**P0 exit:** fixed_point green; multi-file oodac native; emit covers oodac sources; stage-1 usable.

---

# P1 — Beta product surface in `.oo` (R1 parity)

*Define the **beta CLI surface** (CHS-sized), then achieve parity. Out-of-surface stays fail-closed.*

## P1.0 Surface contract

- [ ] **P1.0.1** Freeze beta CLI list (proposal): `check`, `run` (or native-only), `build --target c`, `dump tokens|ast|check`, version, help  
- [ ] **P1.0.2** Explicitly **out of beta** (fail-closed): LSP, pkg, migrate, full WASM product, LLVM product, async/net/crypto (unless already CHS)  
- [ ] **P1.0.3** Write `bootstrap/BETA_SURFACE.md` **or** section in BETA — only if needed; prefer checkboxes here  
- [ ] **P1.0.4** Golden corpus layout: `bootstrap/corpus/{lex,parse,check,typecheck,emit-c,run,build}/pass|fail`  

## P1.1 Lex / tokens (oodac vs host)

- [ ] **P1.1.1** Full CHS lex fail-closed parity (`chs_parity` / dual dump)  
- [ ] **P1.1.2** Unknown char / bad string / bad number fail fixtures  
- [ ] **P1.1.3** Keyword set matches host for beta surface  
- [ ] **P1.1.4** After parity: host `lexer/` can be demoted (not deleted until CLI switches)  

## P1.2 Parse / AST dump

- [ ] **P1.2.1** AST dump parity (span policy documented)  
- [ ] **P1.2.2** All CHS stmt/expr forms parse in oodac  
- [ ] **P1.2.3** Fail fixtures for syntax errors  
- [ ] **P1.2.4** Host `parser/` demote path listed  

## P1.3 Capabilities (default-deny)

- [ ] **P1.3.1** Sealed-effect table parity with host `capabilities/`  
- [ ] **P1.3.2** Net/Fs/Sys/Env param requirements  
- [ ] **P1.3.3** Fail corpus: sealed without cap (exists — expand)  
- [ ] **P1.3.4** Port remaining cap checks from host; dual-engine lock  
- [ ] **P1.3.5** Delete host capabilities only after CLI uses oodac path  

## P1.4 Typecheck

- [ ] **P1.4.1** Inventory host typecheck tests (`typecheck/tests_*`) → oodac corpus gaps  
- [ ] **P1.4.2** Names / scope / immut assign (partial — close gaps)  
- [ ] **P1.4.3** Calls arity/args/returns  
- [ ] **P1.4.4** Ops unary/cmp/logic  
- [ ] **P1.4.5** Struct / field / methods  
- [ ] **P1.4.6** Control flow typing  
- [ ] **P1.4.7** Refinements `Int[lo..hi]`  
- [ ] **P1.4.8** must_use Result/Option  
- [ ] **P1.4.9** Type aliases  
- [ ] **P1.4.10** Full dual-engine typecheck corpus D=0 (maintain)  
- [ ] **P1.4.11** Host `typecheck/` delete gate when oodac is default  

## P1.5 Interpreter / run

- [ ] **P1.5.1** Decide beta: **interpreter in `.oo`** vs **native-only run** (prefer one)  
- [ ] **P1.5.2** If interpreter: port eval builtins CHS set to `.oo`  
- [ ] **P1.5.3** Cap-gated I/O in interpreter parity  
- [ ] **P1.5.4** `main` argv / multi-file import run  
- [ ] **P1.5.5** Fail-closed unsupported ops  
- [ ] **P1.5.6** Host `eval/` delete gate  

## P1.6 Codegen C (product build)

- [ ] **P1.6.1** oodac emit-c **or** pure `.oo` codegen replaces host as default for CHS  
- [ ] **P1.6.2** Link line: gcc + `runtime/chs_rt*.c` only for pure CHS  
- [ ] **P1.6.3** Host FFI path (`libooda`) — quarantine: only for leftover host dumps during alpha  
- [ ] **P1.6.4** Remove host FFI from self-host build path  
- [ ] **P1.6.5** Host `codegen_c/` delete gate  
- [ ] **P1.6.6** Host `codegen/` (LLVM) — beta out or minimal fail-closed  
- [ ] **P1.6.7** Host `codegen_wasm/` — beta out or fail-closed  

## P1.7 CLI driver in `.oo`

- [ ] **P1.7.1** `.oo` CLI: parse args, dispatch check/run/build/dump  
- [ ] **P1.7.2** Version string from pin file (no Rust clap)  
- [ ] **P1.7.3** Exit codes match host policy (0 OK, non-zero ERR)  
- [ ] **P1.7.4** `--json-errors` — port or drop for beta (fail-closed if drop)  
- [ ] **P1.7.5** Help text  
- [ ] **P1.7.6** Install entry runs `.oo` binary not `cargo run`  

## P1.8 Diagnostics / dump

- [ ] **P1.8.1** tokens dump format stable  
- [ ] **P1.8.2** check ERR lines stable enough for qa  
- [ ] **P1.8.3** Host `dump/`, `diagnostics/`, `outline/` — port subset or drop  

**P1 exit:** beta CLI surface runs on stage-1 oodac (+ C) with dual-engine/parity green for that surface.

---

# P2 — Delete Rust by module (R2)

*Order: only after a replacement is default. Each module: parity → switch → delete → RS_COUNT ↓.*

## P2.0 Process

- [ ] **P2.0.1** Per-module checklist template: implement · parity script · switch default · delete · pin  
- [ ] **P2.0.2** Ban silent dual-maintain (old Rust path not called)  
- [ ] **P2.0.3** Track `RS_COUNT` in every PROGRESS pin  

## P2.1 Frontend host modules (delete after P1.1–P1.4)

- [ ] **P2.1.1** Delete `src/lexer/`  
- [ ] **P2.1.2** Delete `src/parser/`  
- [ ] **P2.1.3** Delete `src/ast/` (if only host-needed; else keep until last consumer gone)  
- [ ] **P2.1.4** Delete `src/capabilities/`  
- [ ] **P2.1.5** Delete `src/typecheck/`  
- [ ] **P2.1.6** Delete related tests that only exercise host frontend  

## P2.2 Execution / emit host modules

- [ ] **P2.2.1** Delete `src/eval/`  
- [ ] **P2.2.2** Delete `src/codegen_c/`  
- [ ] **P2.2.3** Delete or quarantine `src/codegen/`  
- [ ] **P2.2.4** Delete or quarantine `src/codegen_wasm/`  
- [ ] **P2.2.5** Delete `src/host_api.rs` / FFI once no libooda  

## P2.3 Tooling modules (beta-out preferred)

- [ ] **P2.3.1** LSP: drop from beta product **or** reimplement in `.oo` (prefer drop) → delete `src/lsp/`  
- [ ] **P2.3.2** pkg: drop or `.oo` → delete `src/pkg/`  
- [ ] **P2.3.3** migrate: drop or `.oo` → delete `src/migrate/`  
- [ ] **P2.3.4** patch: drop or `.oo` → delete `src/patch/`  
- [ ] **P2.3.5** bench: drop or `.oo` → delete `src/bench/`  
- [ ] **P2.3.6** outline / reflect / replay / fmt / context — classify keep-port vs drop  

## P2.4 CLI / lib shell

- [ ] **P2.4.1** Replace `src/cli_parts/*` with `.oo` CLI  
- [ ] **P2.4.2** Delete `src/main.rs`, `src/lib.rs`  
- [ ] **P2.4.3** Delete `tests/*.rs` (json_errors, wasm_host, …) after replaced by corpus/qa  
- [ ] **P2.4.4** Remove `include!` part packs residual (lsp/wasm/…) as modules die  

## P2.5 Host rename hygiene (optional, parallel, low priority)

*Does not remove Rust — only reduces W. Do on touch only.*

- [ ] **P2.5.1** Rename remaining `partNN` in `codegen_wasm/`  
- [ ] **P2.5.2** Rename `lsp/part*`  
- [ ] **P2.5.3** Rename `codegen/part*`  
- [ ] **P2.5.4** Rename `dump/`, `diagnostics/`, `outline/`, `pkg/`, `ast/` packs  
- [ ] **P2.5.5** Prefer **delete** over rename when module is beta-out  

**P2 exit:** no host module still required for beta CLI; RS_COUNT only leftover droppable crates.

---

# P3 — Bootstrap without Rust (R3)

## P3.1 Trusted seed

- [ ] **P3.1.1** Define seed artifact: last known-good **native** `ooda`/`oodac` built from `.oo`+C  
- [ ] **P3.1.2** Seed build reproducible (script + pinned sources)  
- [ ] **P3.1.3** Seed builds stage-1 from clean checkout **without rustc**  
- [ ] **P3.1.4** Document: “builder needs gcc + seed binary only”  

## P3.2 Fixed-point product surface

- [ ] **P3.2.1** stage-1 builds stage-2 for **full beta surface** (not tokens-only)  
- [ ] **P3.2.2** Digest policy: bit-identical or hashed digests (document)  
- [ ] **P3.2.3** Intentional drift fails (already in fixed_point spirit)  
- [ ] **P3.2.4** Cross-machine smoke (optional CI)  

## P3.3 Packaging

- [ ] **P3.3.1** `scripts/release.sh` produces tarball **without** `cargo build`  
- [ ] **P3.3.2** Install path uses prebuilt binary + `install.oo`  
- [ ] **P3.3.3** Website install pins match (openOODA.github.io)  
- [ ] **P3.3.4** No `libooda.a` in pure CHS install  

## P3.4 Remove Cargo from product

- [ ] **P3.4.1** Delete `Cargo.toml` / `Cargo.lock`  
- [ ] **P3.4.2** Delete all remaining `.rs` (**B0**)  
- [ ] **P3.4.3** CI matrix: image **without** Rust; build+test green (**B1**)  
- [ ] **P3.4.4** Dev README: no rustup instructions as primary  

**P3 exit:** B0–B3 true on a clean Linux builder with gcc + seed only.

---

# P4 — Org / ecosystem (B5)

- [ ] **P4.1** `std/` remains `.oo` only; no Rust  
- [ ] **P4.2** `qa/` drives product binary; rewrite any Rust helpers  
- [ ] **P4.3** `docs/` / site pin beta honesty (“no Rust”)  
- [ ] **P4.4** `helloworld` / brand / tree-sitter / vscode — no Rust **requirement** for core product  
- [ ] **P4.5** tree-sitter grammars OK if optional editor support (not compiler critical path)  
- [ ] **P4.6** Legal/LICENSE unchanged; no beta claim early  

---

# P5 — Honesty, entropy, quality rails (always-on)

## P5.1 Entropy & line lock

- [ ] **P5.1.1** O=0 every ship  
- [ ] **P5.1.2** S reported every PROGRESS pin with U/D/F/W/O  
- [ ] **P5.1.3** Never grow monofiles; extract at seams  
- [ ] **P5.1.4** Untested claim → U until fixture lands  

## P5.2 Dual-engine & parity

- [ ] **P5.2.1** Maintain typecheck corpus D=0 while dual engines exist  
- [ ] **P5.2.2** `chs_parity.sh` green for CHS dumps  
- [ ] **P5.2.3** `chs_semantic_parity.sh` as applicable  
- [ ] **P5.2.4** When host deleted, parity becomes stage-N vs stage-N+1 only  

## P5.3 Fail-closed

- [ ] **P5.3.1** Unsupported beta-out features exit non-zero  
- [ ] **P5.3.2** No soft-skip lex/parse/check  
- [ ] **P5.3.3** B4 review before any beta tag  

## P5.4 Runtime C (allowed forever)

- [ ] **P5.4.1** Keep `runtime/chs_rt*.c` tracked (gitignore exceptions)  
- [ ] **P5.4.2** Minimal OS surface: print, str, list, fs, env, process  
- [ ] **P5.4.3** No Rust in runtime  
- [ ] **P5.4.4** Optional: split further only at domain seams ≤250  

---

# P6 — Beta tag (R4)

- [ ] **P6.1** All B0–B5 checked with proof logs  
- [ ] **P6.2** Version: first beta tag scheme (e.g. `0.1.0-beta` / policy forward-only)  
- [ ] **P6.3** GitHub Release + install pin  
- [ ] **P6.4** Public notes: self-hosted; **no Rust in tree**; CHS/beta surface listed  
- [ ] **P6.5** PROGRESS: RS_COUNT=0; S residual only non-beta debt  
- [ ] **P6.6** Do **not** call alpha “beta”  

---

# Workstreams (wide view — parallel where safe)

```text
WS-A  Native self-build     P0.1 → P0.4 → P3.2     [CRITICAL PATH]
WS-B  emit-c coverage       P0.3                     [CRITICAL PATH]
WS-C  Import load           P0.2                     [CRITICAL PATH]
WS-D  Typecheck/cap parity  P1.3–P1.4                [HIGH]
WS-E  Run path              P1.5                     [HIGH]
WS-F  CLI .oo               P1.7                     [HIGH]
WS-G  Delete host modules   P2.*                     [after switch]
WS-H  Beta-out drop         P2.3 tooling             [anytime: reduces mass]
WS-I  Bootstrap/CI no Rust  P3                       [after G]
WS-J  Org pins/docs         P4, P6                   [late]
WS-K  Hygiene/renames       P2.5, P5                 [low; never blocks A]
```

**Parallelism rules:**

- WS-H (drop LSP/pkg from beta) can run **anytime** — cuts future port work.  
- WS-B/C unblock WS-A.  
- WS-G never before switch default.  
- WS-K never steals critical-path tokens for more than a small slice.

---

# Host module kill-list (wide inventory)

| Module | Role | Strategy | Gate |
|--------|------|----------|------|
| `lexer` | stage-0 lex | Replace with oodac tokens | P2.1.1 |
| `parser` | stage-0 parse | oodac ast | P2.1.2 |
| `ast` | host AST types | Keep until last Rust consumer | P2.1.3 |
| `capabilities` | sealed effects | oodac check_caps + expand | P2.1.4 |
| `typecheck` | types | oodac tc_* parity | P2.1.5 |
| `eval` | interpreter | `.oo` eval or native-only | P2.2.1 |
| `codegen_c` | C backend | oodac emit-c | P2.2.2 |
| `codegen` | LLVM | drop beta | P2.2.3 |
| `codegen_wasm` | WASM | drop beta | P2.2.4 |
| `cli_parts` | CLI | `.oo` main | P2.4.1 |
| `lsp` | language server | drop beta | P2.3.1 |
| `pkg` | packages | drop or later | P2.3.2 |
| `migrate` | codemods | drop or later | P2.3.3 |
| `patch` | surgical edit | drop or later | P2.3.4 |
| `bench` | benches | drop or qa | P2.3.5 |
| `dump`/`diagnostics`/`outline` | tooling | subset or drop | P1.8 |
| `fmt`/`reflect`/`replay`/`context`/`em` | misc | classify | P2.3.6 |
| `tests/*.rs` | host tests | corpus + qa | P2.4.3 |
| `runtime/*.c` | **keep** | thin C | P5.4 |

---

# Suggested pin sequence (deep schedule)

| Pin theme | Focus checkboxes | Exit signal |
|-----------|------------------|-------------|
| **α+1** | P0.1.1–P0.1.5 fixed_point green | fixed_point.sh 0 |
| **α+2** | P0.2 import load + fail fixtures | multi-file check/emit without concat |
| **α+3** | P0.3 List/String/Result emit | oodac sources emit |
| **α+4** | P0.4 stage-1 default for check corpus | no `ooda run` needed for check |
| **α+5** | P1.4 typecheck gap close | dual-engine full corpus |
| **α+6** | P1.5 run path decision + MVP | run CHS programs via non-Rust |
| **α+7** | P1.7 `.oo` CLI | `ooda` binary from `.oo`+C |
| **α+8** | P2.1 delete frontend Rust | RS_COUNT big drop |
| **α+9** | P2.2 delete eval/codegen_c | RS_COUNT big drop |
| **α+10** | P2.3 drop tooling Rust | RS_COUNT |
| **α+11** | P3 bootstrap no cargo | B0–B3 |
| **β0** | P6 tag | B0–B5 |

*(Exact version numbers TBD; always forward.)*

---

# Near-term todo (next 5 ships only)

*Copy into PROGRESS “Next” when working.*

1. [ ] Minimal repro + fix list-pack C types (P0.1.1–P0.1.2)  
2. [ ] fixed_point.sh green (P0.1.5)  
3. [ ] Import-aware check/emit load (P0.2.2–P0.2.3)  
4. [ ] emit-c List[Int]/String] + fixtures (P0.3.8)  
5. [ ] stage-1 oodac check corpus without interpreter (P0.4.2)  

---

# Anti-todo (do not)

- [ ] ❌ Delete all `.rs` before fixed_point green  
- [ ] ❌ Claim beta while Cargo still required  
- [ ] ❌ Weaken fixed_point / parity to fake green  
- [ ] ❌ Port LSP/pkg before self-host spine  
- [ ] ❌ Rename all partNN as main beta program  
- [ ] ❌ Grow DESIGN / invent new language for bootstrap  
- [ ] ❌ New Rust-only features on the critical path  
- [ ] ❌ Check in huge generated C as “self-host done”  

---

# Metrics dashboard (every pin)

```text
RS_COUNT:   ___
OO_COUNT:   ___
O:          ___   (must be 0)
S: U+D+F+W+O = ___
fixed_point:  green|red
c_emit_smoke: green|red
dual_engine_tc: D=___
stage-1 binary: yes|no
default driver: host|oodac|mixed
beta gates: B0_B1_B2_B3_B4_B5 = 0/1 each
```

---

# Definition of “Rust free”

All true:

1. **B0** zero `.rs` in `ooda` product tree  
2. **B1** CI/build without rustc  
3. **B2** `.oo` compiler fixed-point on beta surface  
4. **B3** release binary from that pipeline  
5. **B4** fail-closed honesty  
6. **B5** org siblings non-Rust product  

Until then: **alpha**, report RS_COUNT, work the critical path.

---

*This map is operational, not architecture. When reality changes (e.g. fixed_point green), recompute baseline and tick boxes — do not invent a second plan.*
