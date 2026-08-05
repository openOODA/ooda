# File-size split plan (≤250 lines) — functional boundaries only

**Status:** Plan only — execute after current Gemini turn finishes.  
**Rule:** Owned source ≤ **250 lines** per file (`scripts/check_file_lines.sh`).  
**Entropy:** \(O\) = oversize file count; good ships drive **\(O \downarrow\)** (`TOOLS.md`).  
**Non-negotiable:** Splits are **functional and first-principles**, never “every N lines.”

Generated excluded: `oodac/main.c`, `oodac/oodac2.c`, `*.oo.c`, `*.oo.bin`, `target/`, `dist/`.

---

## First principles (read before any extract)

### 1. A module is a *job*, not a line budget

Cut only where a competent compiler engineer would name a crate/package:

| Question | Must answer “yes” |
|----------|-------------------|
| Does this unit have **one responsibility** in the pipeline or language model? | |
| Can you state its **inputs → outputs** without referring to “the rest of main.oo”? | |
| Would a **new reader** know where a bug in behavior X lives? | |
| Do dependencies point **downstream only** (lex ↛ depends on typecheck)? | |

If the only reason to cut is “file hit 250,” **keep cutting conceptually** until the seam is a real interface — then place the file boundary there. If that unit is still >250, **decompose the unit further by sub-responsibility**, not by line number.

### 2. Pipeline is the primary axis (universal compiler shape)

oodac’s own section markers already encode this (do not invent a second architecture):

```text
source text
    → Lex          (characters → token validity / positions)
    → Tokens       (stable token stream representation + dump)
    → Parse / AST  (tokens → structure: items, stmts, exprs)
    → Check        (structure + sealed capabilities / program shape)
    → Typecheck    (types, names, control, structs, refinements, …)
    → Emit (C)     (tokens/structure → C text)
CLI (main) only dispatches commands onto this pipeline.
```

**File tree must mirror this pipeline.** Do not mix lex helpers into typecheck files or emit into parse.

### 3. Within a stage, split by *language semantics*, not chronology

Especially typecheck: group by **what fact about the program is being enforced**, aligned with DESIGN pillars and grammar:

| Semantic domain | Enforces | Examples in monofile today |
|-----------------|----------|----------------------------|
| **Names / bindings** | What exists; scopes | `typecheck_undefined_vars`, `binds_has`, `scope_has`, `drop_depth`, `is_known_name` |
| **Type lattice / lits** | What a type is; lit↔ann | `lit_token_type`, `types_compatible`, `resolve_type_alias`, `combine_binop_types`, `typecheck_ann_and_return_lits` |
| **Calls** | Arity, arg/ret of calls | `typecheck_call_*`, `resolve_arg_type`, `arity_get`, `build_fn_ret_table`, `typecheck_let_ann_call_init` |
| **Assignment / mutability** | Store rules | `typecheck_immut_assign`, `typecheck_mut_assign_types`, `typecheck_return_and_assign_calls`, `is_mut_binding` |
| **Operators** | Op well-typedness / rejects | unary bang/minus, cmp, logic binop, shift/amp/pipe rejects |
| **Structs / fields / methods** | Nominal fields & receivers | `build_struct_field_table`, `field_chain_*`, `typecheck_field_*`, `typecheck_struct_lit_inits` |
| **Control flow** | Cond types; branch consistency | `typecheck_if_while_lit_cond`, `typecheck_control_flow_branches` |
| **Refinements** | `Int[lo..hi]` etc. | `typecheck_refinements`, `parse_int` (as refine helper) |
| **Effects / caps** | Sealed I/O requires caps | `check_function`, `is_sealed_*` (DESIGN pillar 1) |
| **Must-use / returns** | Result discipline | `typecheck_must_use_result`, `typecheck_missing_return` |

A typecheck file named `tc_misc.oo` is a **smell** — it is a junk drawer, not a principle. Prefer the domains above; if something does not fit, name the new domain, don’t dump it.

### 4. Function boundary before file boundary

1. **Never** split mid-`fn` at an arbitrary line.  
2. If `fn f` is >250 (or bloated): extract **named** helpers that mean something  
   (`lex_string_lit`, `lex_number`, `check_call_arity_at`) — not `f_part2`.  
3. Only after helpers exist do you move a **closed set** of fns that share one job into a module.  
4. Public surface of a module should be small: **stage entry + few pure helpers** others need.

### 5. Dependency rule (acyclic)

```text
cli → {lex, tokens, parse, check, typecheck_*, emit}
typecheck_* → may use: tokens representation, parse navigators (tok field accessors), type lattice helpers
emit → may use: tokens / light parse navigation; must not call typecheck
parse → may use: tokens; must not call typecheck or emit
lex → pure over String; no parse/tc/emit
```

If an extract forces a reverse dependency, the **seam is wrong** — redesign the interface (e.g. pass token list + indexes), don’t “just move lines.”

### 6. Dual-engine honesty is a seam constraint

Stage-0 Rust modules already follow pipeline names (`lexer`, `parser`, `typecheck`, `eval`, `codegen_*`).  
**oodac modules should map to the same conceptual stages** so dual-engine work has an obvious peer, not a random `.oo` shard.

### 7. 250 is a *pressure*, not the *reason*

- Cap forces modularity (SPEC § LLM context).  
- **Reason** for each file is the job in §1–3.  
- Reject PRs/rotations that only chop for green checker without a one-line job statement in the module header.

---

## Anti-patterns (explicit)

| Forbidden | Why |
|-----------|-----|
| Split at line 250 / 500 / 1000 | No meaning; breaks fns mid-thought |
| `main_1.oo` … `main_4.oo` | Chronological dump |
| `tc_misc` / `utils` catch-alls | Entropy dump; grows forever |
| Moving half a typecheck domain to “make room” | Breaks cohesion |
| New behavior only in the leftover monofile | Restores the problem |
| Reverse imports (lex imports tc) | Architecture lie |

---

## Current baseline (recompute after Gemini)

```bash
cd /home/jeryd/Projects/openOODA/ooda
./scripts/check_file_lines.sh
# O=24 at plan refresh; oodac/main.oo ~7.7k+ and may still grow under Gemini
```

| Priority | Path | Role (functional) |
|---------:|------|-------------------|
| **P0** | `oodac/main.oo` | Whole R1 pipeline in one file |
| **P1** | `src/{typecheck,eval,codegen_*,main,parser,…}.rs` | Stage-0 host — same pipeline jobs |
| **P2** | large tests/scripts/runtime | Harness / RT — split by **suite domain** or **RT subsystem** |

**Order:** P0 first (product path). P1 only when that host file is touched or after oodac stages exist as peers.

---

## P0 — `oodac/main.oo` target architecture

### Module map (each file = one job)

```text
oodac/
  main.oo                 # CLI dispatch only (tokens|ast|check|build|emit-c)
  lex.oo                  # Character scanner: lex_all (+ pure lex helpers if extracted)
  token_stream.oo         # Token materialization: collect_tokens, tok_line, field_at, n_toks, …
  token_dump.oo           # Human/tool dump path: dump_tokens, emit_tok, keyword_kind
  parse_item.oo           # Item-level: parse_fn_item, parse_and_print_ast, skip_balanced
  parse_stmt.oo           # Statement structure: block/stmt dump, skip_stmt, count_stmts, indent
  parse_expr.oo           # Expression grammar: bp/primary/method chain, bin_bp, escape_dump
  check_drive.oo          # check command orchestration: run_check_from_src
  check_caps.oo           # Sealed-effect capability checking (DESIGN caps pillar)
  tc_types.oo             # Type lattice: aliases, lit types, compatibility, binop type combine
  tc_scope.oo             # Bindings/scopes/env tables for names
  tc_names.oo             # Undefined / known-name enforcement
  tc_calls.oo             # Call arity, arg types, call-vs-lit/op interactions, fn ret table
  tc_assign.oo            # Let/assign/return store rules + mut
  tc_ops.oo               # Unary/cmp/logic + rejected operators
  tc_struct.oo            # Struct tables, field chains, methods, struct lits
  tc_control.oo           # if/while conditions + branch typing
  tc_refine.oo            # Refinement types Int[lo..hi] etc.
  tc_effects.oo           # must_use Result, missing return (completion/effect of values)
  c_emit.oo               # C backend entry + fn/block/stmt/preamble
```

**Header required** at top of every new module (one short block comment):

```text
// job: <one sentence>
// in:  <types/values>
// out: <types/values or side-effect>
// stage: lex | tokens | parse | check | typecheck | emit | cli
```

If you cannot fill that header honestly, **do not create the file**.

### How monofile maps today (anchors, not cut lines)

| Stage | Existing markers / owners | Typical entry fns |
|-------|---------------------------|-------------------|
| CLI | top of file | `main` |
| Lex | `// ===== Lex all` | `lex_all` |
| Tokens | `// ===== Token dump` | `dump_tokens`, `collect_tokens`, `emit_tok`, `keyword_kind` |
| Parse | `// ===== Real AST dump` | `dump_ast_from_src`, `parse_*`, `skip_*`, `print_expr_bp`, … |
| Check + caps | `// ===== Real capability + structure check` | `run_check_from_src`, `check_function`, `is_sealed_*` |
| Typecheck | `// ===== R1 typecheck slice` | all `typecheck_*`, type lattice, struct/field helpers, infer_* |
| Emit | late file | `c_emit_stream`, `c_emit_fn/block/stmt`, `c_emit_preamble` |

Line numbers **shift**; re-find by **marker + `pub fn` name**, never by stale L#### alone.

### Oversized *functions* — split by sub-responsibility

When a single `pub fn` exceeds ~250 or is clearly multi-job:

| Function (today) | First-principles decomposition |
|------------------|--------------------------------|
| `dump_tokens` / `collect_tokens` | Separate **string lit**, **number**, **ident/keyword**, **punct** scanners; shared cursor state helpers |
| `parse_primary_dump` | **atom**, **paren/group**, **call suffix**, **field/method suffix** |
| `typecheck_ann_and_return_lits` | **annotation check** vs **return-lit check** vs **alias resolve** path |
| `typecheck_call_arg_lits` | **resolve callee**, **each arg**, **arity** (compose `typecheck_call_arity`) |
| `typecheck_refinements` | **parse bound**, **check let**, **check assign/return** |
| `typecheck_field_method` | **resolve receiver type**, **method arity table**, **arg check** |

Helpers stay in the **same stage module** until that module’s job is full — then promote a sub-module that is still one semantic noun (`token_scan_string.oo` only if string lex is a real subsystem).

### Phased extraction (each phase = one pipeline seam)

| Phase | Functional goal | Move (closed set) | Prove with |
|------:|-----------------|-------------------|------------|
| **0** | Baseline + no growth | Nothing — measure \(O\), ratchet | `check_file_lines.sh` |
| **1** | **Lex stage** standalone | `lex_all` + pure lex helpers only | lex fail fixtures; tokens/ast/check still green |
| **2** | **Token stream** vs **token dump** | Materialize stream vs print dump | `tokens` cmd + consumers of `collect_tokens` |
| **3** | **Parse: items** | item-level parse only | `ast` on fn-shaped programs |
| **4** | **Parse: stmt / expr** | stmt module then expr module (expr may precede stmt if deps require) | nested expr/stmt ast fixtures |
| **5** | **Check drive + caps** | orchestration vs sealed caps (two files, two jobs) | fail-closed sealed; structure check |
| **6** | **Type lattice + scope** | `tc_types` + `tc_scope` first (foundations) | ann/return lit fixtures |
| **7** | **Typecheck by domain** | names → calls → assign → ops → struct → control → refine → effects | corpus **per domain** dual-engine |
| **8** | **Emit** | all `c_emit_*` | emit-c + build smoke |
| **9** | **CLI-only main** | `main` dispatch only ≤250 | fixed_point / CHS scripts |

**Phase order is dependency order**, not “easiest line chop.”  
Typecheck domains (phase 7) may be multiple rotations — **one domain per rotation** is correct Power Law.

### Tests (functional, not theater)

- Fixtures must fail if the **job** of the module regresses (immune rail).  
- Prefer corpus paths that match the domain (`typecheck/fail/*` for tc_*, etc.).  
- Dual-engine on the same fixture when the stage has a stage-0 peer.  
- No “split done” without green suite for that stage.

### Multi-file wiring

Use the language’s real module mechanism (filesystem modules / imports per DESIGN–SPEC).  
If oodac currently runs only as a single file: temporary **concat for ship** is allowed only if:

1. Source of truth remains **split files** with real jobs, and  
2. Concat is a **build step**, not the editing surface, and  
3. Goal is still true multi-file load — concat is residual host-like debt.

Do not pretend concat is the architecture.

---

## P1 — Stage-0 Rust (same principles)

Split **along the same pipeline and semantic domains**, matching existing module names where they already exist:

| Current monofile | Functional split (examples) |
|------------------|-----------------------------|
| `typecheck.rs` | types / scope / names / calls / assign / ops / struct / control / refine — **same domains as oodac** |
| `eval.rs` | values / expr / stmt / call / caps effects |
| `codegen_c.rs` / `codegen_wasm.rs` | preamble/types, expr, stmt, fn, runtime glue |
| `main.rs` | one module per CLI command surface |
| `parser.rs` | item / stmt / expr (grammar nonterminals) |
| `capabilities.rs` | per cap kind or check vs env |

Do not create `typecheck_part2.rs`.  
Do not grow host files; if you must touch one >250, **extract a domain first** in the same rotation.

---

## P2 — Tests, scripts, runtime

| Asset | Functional split |
|-------|------------------|
| `tests/json_errors_golden.rs` | By **diagnostic class** (or data file + ≤250 runner) |
| `tests/wasm_host.rs` | By **host feature** (lists, strings, control, …) |
| `scripts/chs_parity.sh` | By **parity concern** (frontend fixed-point vs semantic) |
| `install/install.oo` | By **install phase** (fetch, place, verify) |
| `runtime/chs_rt.c` | By **runtime subsystem** (strings, lists, I/O, process) |

---

## Lock policy

1. **While \(O > 0\):** `./scripts/check_file_lines.sh --ratchet` — no oversize growth, no new oversize.  
2. **Goal:** strict checker → \(O = 0\).  
3. **PROGRESS:** report \(O\) and which **stage/domain** moved.  
4. Reject work that raises \(O\) or adds features only into the monofile leftover.

---

## Post-Gemini checklist

- [ ] Re-run checker; record \(O\)  
- [ ] Clean non-product scratch  
- [ ] Phase 0 ratchet green (stop growth)  
- [ ] Phase 1–2: **lex** then **tokens** (pipeline head)  
- [ ] Phase 3–5: **parse** then **check/caps**  
- [ ] Phase 6–7: **tc foundations** then **one domain per rotation**  
- [ ] Phase 8–9: **emit** then thin **CLI**  
- [ ] P1 host only on touch, same domain rules  
- [ ] \(O = 0\) strict  

---

## Review gate (every extract PR/rotation)

Before Lock, answer in PROGRESS (≤3 bullets):

1. **Job** of each new/changed module (one sentence).  
2. **Stage** in the pipeline.  
3. **Why this seam** (first principle — not “file was long”).

If (3) is only line count → **rework the split**.

---

*Process/bootstrap plan. Product truth: `DESIGN.md`. Beta: `BETA.md`. Line Lock: `scripts/check_file_lines.sh`.*
