# Complete split plan — all owned source ≤256 lines

**Status:** Full inventory + functional target map. Execute after each rotation’s probes.  
**Cap:** 256 lines per owned `.oo` / `.rs` / hand `.c`/`.h` / `.sh`.  
**Lock:** `scripts/check_file_lines.sh` (strict → O=0; `--ratchet` while O>0).  
**Entropy:** \(O\) = oversize file count.  
**Law:** Splits only at **functional seams** (pipeline + language domain). Never mid-fn line chops.

Generated **excluded** from cap: `oodac/main.c`, `oodac/oodac2.c`, `*.oo.c`, `*.oo.bin`, `target/`, `dist/`.

---

## First principles (every extract)

1. **One job per file** — name the responsibility; fill header `job / in / out / stage`.  
2. **Pipeline primary axis:** `cli → lex → tokens → parse → check → typecheck → emit`.  
3. **Typecheck secondary axis:** names · types · calls · assign · ops · struct/field · control · refine · effects.  
4. **Function before file** — oversized `fn` → named sub-jobs, then move a closed set.  
5. **Deps acyclic** — later stages may use earlier; never reverse.  
6. **256 is pressure** — seam reason is never “file was long.”  
7. **Peer stage-0** — oodac modules should map to host module *concepts* for dual-engine work.

### Required module header

```text
// job: <one sentence>
// in:  <…>
// out: <…>
// stage: cli | lex | tokens | parse | check | typecheck | emit | host | test | install | runtime
```

---

## Baseline (recompute each Observe)

```bash
./scripts/check_file_lines.sh   # O=24 as of this plan; main.oo ~7297
```

| Done (≤256) | Remaining monofile weight |
|-------------|---------------------------|
| `lex.oo`, `token_emit.oo`, `token_fmt.oo`, `check_caps.oo` | `oodac/main.oo` + 23 other oversize paths |

---

# Part A — `oodac/main.oo` (P0) — complete target tree

## A.0 Target tree (every file ≤256)

```text
oodac/
  main.oo                 # CLI only (≤256)
  lex.oo                  # DONE — lex_all
  token_emit.oo           # DONE — emit_tok, keyword_kind
  token_fmt.oo            # DONE — tokenize_lines, tok_line, field_at
  token_scan_string.oo    # string-lit scan helpers (from dump/collect)
  token_scan_number.oo    # number/float scan helpers
  token_scan_ident.oo     # ident + keyword path
  token_scan_punct.oo     # operators/punct multi-char
  token_scan_ws.oo        # ws + line comments (shared)
  token_dump.oo           # dump_tokens driver (thin loop)
  token_stream.oo         # collect_tokens driver (thin loop)
  parse_drive.oo          # dump_ast_from_src, parse_and_print_ast, n_toks
  parse_item.oo           # parse_fn_item, skip_balanced
  parse_stmt.oo           # block/stmt dump, skip_stmt, count_stmts, indent
  parse_expr.oo           # expr bp, skip/print, bin_bp, bin_op_name
  parse_primary.oo        # parse_primary_dump + method chain/from_var, escape_dump
  parse_skip.oo           # skip_primary, skip_expr_bp (navigation only)
  check_drive.oo          # run_check_from_src orchestration
  check_caps.oo           # DONE — check_function, is_sealed_*, skip_until_semi
  tc_types.oo             # type lattice
  tc_scope.oo             # binds/scope/env
  tc_names.oo             # undefined / known names
  tc_calls.oo             # call arity/args/call-op interactions (+ helpers)
  tc_assign.oo            # immut/mut assign, return/assign calls
  tc_ops.oo               # unary, cmp, logic, rejected ops
  tc_struct.oo            # struct tables + field chains + methods + struct lits
  tc_control.oo           # if/while cond + branch typing
  tc_refine.oo            # refinements + parse_int
  tc_effects.oo           # must_use, missing_return
  tc_infer.oo             # lit env + typed expr inference helpers
  c_emit.oo               # c_emit_* (grow carefully; split if >256)
```

Wire: `import "….oo";` from `main.oo` (and between modules only **down** the DAG).

**Self-host note:** stage-0 `ooda run` expands imports. oodac `emit-c`/`check` on a *path* may still be single-file — track as residual; fix with import-aware load or build-time concat **only as temporary debt**.

---

## A.1 Function → module assignment (current monofile)

Line ranges shift; **bind by `pub fn` name**.

### CLI — `main.oo`

| Fn | ~Lines | Notes |
|----|-------:|-------|
| `main` | 83 | Args + dispatch only after extracts |

### Lex — `lex.oo` ✅

| Fn | Status |
|----|--------|
| `lex_all` | Done |

### Tokens — emit/fmt ✅; scan/dump/stream TODO

| Fn | ~Lines | Target module | Job |
|----|-------:|---------------|-----|
| `emit_tok` | 4 | `token_emit.oo` ✅ | print one token |
| `keyword_kind` | 24 | `token_emit.oo` ✅ | ident → KW_* |
| `tokenize_lines` | 13 | `token_fmt.oo` ✅ | src → lines via collect |
| `tok_line` | 4 | `token_fmt.oo` ✅ | encode token line |
| `field_at` | 29 | `token_fmt.oo` ✅ | decode field |
| `dump_tokens` | **404** | **split first** → `token_dump.oo` + scan_* | validate via lex_all; emit stream |
| `collect_tokens` | **366** | **split first** → `token_stream.oo` + scan_* | same scan, store lines |

**Mandatory internal split before file extract** (shared jobs for dump + collect):

| Helper job | Responsibility | Used by |
|------------|----------------|---------|
| `scan_ws_comment` | space/tab/nl + `//` comments; advance i/line/col | dump, collect |
| `scan_string_lit` | `"…"` + escapes → STRING token text | dump, collect |
| `scan_number` | int/float + `..` range boundary rules | dump, collect |
| `scan_ident_or_kw` | ident run + `keyword_kind` | dump, collect |
| `scan_punct` | multi-char ops (`<=`,`&&`,`..=`,…) + singles | dump, collect |

Drivers after helpers exist:

- `dump_tokens`: probe `lex_all` → loop → dispatch scan_* → `emit_tok`  
- `collect_tokens`: loop → dispatch scan_* → append `tok_line`  

Each scan_* module ≤256; drivers thin.

### Parse

| Fn | ~Lines | Target | Job |
|----|-------:|--------|-----|
| `dump_ast_from_src` | 4 | `parse_drive.oo` | entry |
| `parse_and_print_ast` | 85 | `parse_drive.oo` | program walk |
| `n_toks` | 4 | `parse_drive.oo` | len |
| `parse_fn_item` | 230 | `parse_item.oo` | fn item nonterminal |
| `skip_balanced` | 22 | `parse_item.oo` | brace/paren skip |
| `parse_block_dump` | 66 | `parse_stmt.oo` | block |
| `count_stmts` | 26 | `parse_stmt.oo` | |
| `skip_stmt` | 54 | `parse_stmt.oo` | |
| `indent` | 10 | `parse_stmt.oo` | dump formatting |
| `parse_stmt_dump` | 92 | `parse_stmt.oo` | stmt nonterminal |
| `parse_expr_dump` | 4 | `parse_expr.oo` | |
| `parse_expr_bp` | 14 | `parse_expr.oo` | Pratt entry |
| `print_expr_bp` | 74 | `parse_expr.oo` | |
| `bin_bp` | 10 | `parse_expr.oo` | precedence table |
| `bin_op_name` | 16 | `parse_expr.oo` | |
| `skip_expr_bp` | 27 | `parse_skip.oo` | navigate without print |
| `skip_primary` | 151 | `parse_skip.oo` | |
| `parse_primary_dump` | **293** | **split** → `parse_primary.oo` | atoms + call/field suffix |
| `parse_method_from_var` | 52 | `parse_primary.oo` | |
| `parse_method_chain` | 4 | `parse_primary.oo` | |
| `escape_dump` | 21 | `parse_primary.oo` | string escape for dump |

`parse_primary_dump` sub-jobs if still >256: **atom**, **call-args**, **field/method suffix**.

### Check

| Fn | ~Lines | Target | Job |
|----|-------:|--------|-----|
| `run_check_from_src` | 94 | `check_drive.oo` | orchestrate structure + caps + typecheck hooks |
| `check_function` + `is_sealed_*` + `skip_until_semi` | — | `check_caps.oo` ✅ | sealed effects |

### Typecheck — by language domain

#### `tc_types.oo` — type lattice

| Fn | ~Lines |
|----|-------:|
| `resolve_type_alias` | 38 |
| `lit_token_type` | 20 |
| `types_compatible` | 15 |
| `is_type_binop` | 16 |
| `combine_binop_types` | 31 |
| `typecheck_ann_and_return_lits` | **349** → split: **ann path** / **return path** / shared alias |

#### `tc_scope.oo` — bindings

| Fn | ~Lines |
|----|-------:|
| `binds_has` | 7 |
| `scope_has` | 12 |
| `drop_depth` | 62 |
| `env_lookup_type` | 38 |
| `is_mut_binding` | 24 |
| `build_pure_lit_env` | 168 |
| `build_fn_ret_table` | 44 |

#### `tc_names.oo`

| Fn | ~Lines |
|----|-------:|
| `is_known_name` | 75 |
| `typecheck_undefined_vars` | 132 |

#### `tc_calls.oo`

| Fn | ~Lines |
|----|-------:|
| `typecheck_call_arg_lits` | **326** → resolve callee / each arg / compose arity |
| `resolve_arg_type` | 50 |
| `nth_csv` | 25 |
| `arity_get` | 52 |
| `typecheck_call_arity` | 154 |
| `typecheck_call_binop_lits` | 99 |
| `typecheck_call_logic_lits` | 102 |
| `typecheck_call_order_lits` | 134 |
| `typecheck_call_eq_lits` | 96 |
| `typecheck_let_ann_call_init` | 91 |
| `typecheck_return_and_assign_calls` | 193 |

If `tc_calls.oo` would exceed 256 after move: split **`tc_call_arity.oo`** vs **`tc_call_expr.oo`** (call-in-expr checks).

#### `tc_assign.oo`

| Fn | ~Lines |
|----|-------:|
| `typecheck_immut_assign` | 71 |
| `typecheck_mut_assign_types` | 210 |

#### `tc_ops.oo`

| Fn | ~Lines |
|----|-------:|
| `typecheck_unary_bang_lit` | 57 |
| `is_value_token_kind` | 17 |
| `typecheck_unary_minus_lit` | 70 |
| `typecheck_cmp_numeric_lits` | 59 |
| `typecheck_reject_shift_ops` | 50 |
| `typecheck_reject_amp_pipe_binop` | 39 |
| `typecheck_logic_binop_lits` | 120 |

#### `tc_struct.oo`

| Fn | ~Lines |
|----|-------:|
| `build_struct_field_table` | 64 |
| `struct_fields_blob` | 39 |
| `struct_field_type` | 62 |
| `struct_has_field` | 60 |
| `is_known_struct_type` | 7 |
| `field_chain_end` | 41 |
| `field_access_type_at` | 15 |
| `field_chain_type` | 51 |
| `is_field_chain_span` | 29 |
| `count_paren_args` | 40 |
| `method_expected_args` | 41 |
| `is_list_type_name` | 23 |
| `typecheck_field_method` | 237 |
| `typecheck_field_assign` | 87 |
| `typecheck_field_binop_uses` | 79 |
| `typecheck_struct_lit_inits` | 174 |

If over 256: **`tc_struct_table.oo`** (tables) vs **`tc_struct_use.oo`** (field/method/lit checks).

#### `tc_control.oo`

| Fn | ~Lines |
|----|-------:|
| `typecheck_if_while_lit_cond` | 158 |
| `typecheck_control_flow_branches` | 94 |

#### `tc_infer.oo`

| Fn | ~Lines |
|----|-------:|
| `infer_pure_lit_expr_type` | 6 |
| `atom_type_with_env` | 22 |
| `infer_typed_expr_type` | 42 |
| `infer_typed_expr_type_exact` | 46 |

#### `tc_refine.oo`

| Fn | ~Lines |
|----|-------:|
| `parse_int` | 28 |
| `typecheck_refinements` | **270** → parse bounds / check let / check assign·return |

#### `tc_effects.oo`

| Fn | ~Lines |
|----|-------:|
| `typecheck_must_use_result` | 66 |
| `typecheck_missing_return` | 101 |

### Emit — `c_emit.oo` (currently small; keep ≤256)

| Fn | ~Lines |
|----|-------:|
| `c_emit_stream` | 22 |
| `c_emit_fn` | 43 |
| `c_emit_block` | 19 |
| `c_emit_stmt` | 34 |
| `c_emit_preamble` | 35 |

When emit grows: split **preamble/types**, **stmt**, **expr** by codegen job — not by date.

---

## A.2 Ordered execution waves (oodac)

| Wave | Functional goal | Modules | Exit criteria |
|-----:|-----------------|---------|---------------|
| **0** | No growth | — | `--ratchet` green on oodac |
| **1** | Lex | `lex.oo` | ✅ Done |
| **2a** | Token helpers | `token_emit`, `token_fmt` | ✅ Done |
| **2b** | Shared scan kernel | `token_scan_{ws,string,number,ident,punct}` | dump+collect call helpers; tokens cmd green |
| **2c** | Token drivers | `token_dump`, `token_stream` | both ≤256; parity with stage-0 tokens sample |
| **3** | Caps check | `check_caps` | ✅ Done |
| **4** | Check drive | `check_drive` | check orchestration only |
| **5** | Parse drive + item | `parse_drive`, `parse_item` | `ast` on fn programs |
| **6** | Parse stmt / skip / expr / primary | `parse_stmt`, `parse_skip`, `parse_expr`, `parse_primary` | nested ast fixtures |
| **7** | TC foundations | `tc_types`, `tc_scope`, `tc_infer` | ann/return lit fixtures |
| **8** | TC domains (one/rotation) | names → calls → assign → ops → struct → control → refine → effects | corpus per domain dual-engine |
| **9** | Emit | `c_emit` (+ splits if grown) | emit-c smoke |
| **10** | CLI only | `main.oo` ≤256 | fixed_point / CHS scripts; **O contributes −1** |

**Power law:** one domain or one stage per rotation when large.

---

# Part B — Stage-0 host (P1) — all oversize `.rs`

Same pipeline. Split when **touching** the file or after oodac peer exists. Do not grow.

| File | ~Lines | Functional modules (target) |
|------|-------:|-------------------------------|
| `src/typecheck.rs` | 4189 | `typecheck/{mod,types,scope,names,calls,assign,ops,struct_fields,control,refine,effects,util}.rs` — **mirror oodac tc_*** |
| `src/eval.rs` | 2564 | `eval/{mod,value,cap,expr,stmt,call,runtime}.rs` |
| `src/codegen_wasm.rs` | 2314 | `codegen_wasm/{mod,types,expr,stmt,fn,host,strings}.rs` |
| `src/codegen_c.rs` | 1645 | `codegen_c/{mod,sealed,expr,stmt,fn,link,runtime}.rs` |
| `src/main.rs` | 1372 | `cli/{mod,run,check,build,dump,em,version}.rs` or one file per subcommand |
| `src/codegen.rs` | 1043 | by backend dispatch vs shared IR helpers |
| `src/lsp.rs` | 1041 | `lsp/{mod,server,hover,complete,diag}.rs` |
| `src/parser.rs` | 1029 | `parser/{mod,item,stmt,expr,ty}.rs` (grammar nonterminals) |
| `src/capabilities.rs` | 999 | per cap kind + check vs env (`caps/{net,fs,sys,env,check}.rs`) |
| `src/migrate.rs` | 827 | one module per migrate codemod class |
| `src/patch.rs` | 657 | parse patch / apply / validate |
| `src/dump.rs` | 507 | dump tokens vs ast vs other formats |
| `src/pkg.rs` | 423 | resolve / lock / fetch concerns |
| `src/lexer.rs` | 397 | already one stage — split only if scanner classes (string/number/ident) |
| `src/outline.rs` | 391 | outline collect vs format |
| `src/ast.rs` | 338 | types only stay; move large impl/helpers out if needed |
| `src/diagnostics.rs` | 323 | emit JSON vs human; codes table |
| `src/bench.rs` | 298 | bench harness vs cases |

**Host tests inside modules:** `mod tests` in-file is fine if file stays ≤256; else `tests/` or `*_test` modules by domain.

---

# Part C — Tests, install, scripts, runtime (P2)

| File | ~Lines | Functional split |
|------|-------:|------------------|
| `tests/json_errors_golden.rs` | 4362 | **Data vs runner:** golden tables as `.json`/`.oo` fixtures; runner ≤256. Or split by diagnostic class files |
| `tests/wasm_host.rs` | 1678 | By host feature: lists, strings, control, floats, bools, … |
| `install/install.oo` | 448 | Phases: `install_{fetch,place,verify,pin}.oo` + thin driver |
| `runtime/chs_rt.c` | 358 | `chs_rt_{str,list,io,process}.c` + thin `chs_rt.c` umbrella if link allows |
| `scripts/chs_parity.sh` | 314 | `scripts/parity.d/` fragments by concern (frontend FP vs semantic) + thin driver |

---

# Part D — Global order of battle

```text
1. Finish oodac tokens (wave 2b–2c)     ← unblocks readable frontend
2. oodac parse (waves 5–6)
3. oodac typecheck domains (waves 7–8) ← largest honesty surface
4. oodac emit + thin main (waves 9–10)
5. Host typecheck/eval/codegen on touch or after peer stage exists
6. Tests/install/runtime/scripts      ← when they block Lock O=0
```

**O=0** only when Part A–C all clear (or exceptions explicitly revoked in RULES — none today).

---

# Part E — Per-extract checklist

- [ ] Job header written (not “part of main”)  
- [ ] Closed fn set; no mid-fn cut  
- [ ] Deps only downward  
- [ ] `wc -l` ≤256 for every new/changed owned file  
- [ ] `./scripts/check_file_lines.sh --ratchet`  
- [ ] Probes: pass + fail for that stage/domain  
- [ ] Dual-engine if stage has host peer  
- [ ] PROGRESS: \(O\), \(\Delta\), stage name  

---

# Part F — Anti-goals

- Line-number shards (`main_2.oo`)  
- `utils` / `misc` junk drawers  
- Growing any oversize file  
- Deleting `.rs` to fake O without `.oo` ownership  
- Claiming split done without fail+pass rails  

---

## Snapshot: current O=24 list

| Lines | Path | Part |
|------:|------|------|
| 7297 | `oodac/main.oo` | A |
| 4362 | `tests/json_errors_golden.rs` | C |
| 4189 | `src/typecheck.rs` | B |
| 2564 | `src/eval.rs` | B |
| 2314 | `src/codegen_wasm.rs` | B |
| 1678 | `tests/wasm_host.rs` | C |
| 1645 | `src/codegen_c.rs` | B |
| 1372 | `src/main.rs` | B |
| 1043 | `src/codegen.rs` | B |
| 1041 | `src/lsp.rs` | B |
| 1029 | `src/parser.rs` | B |
| 999 | `src/capabilities.rs` | B |
| 827 | `src/migrate.rs` | B |
| 657 | `src/patch.rs` | B |
| 507 | `src/dump.rs` | B |
| 448 | `install/install.oo` | C |
| 423 | `src/pkg.rs` | B |
| 397 | `src/lexer.rs` | B |
| 391 | `src/outline.rs` | B |
| 358 | `runtime/chs_rt.c` | C |
| 338 | `src/ast.rs` | B |
| 323 | `src/diagnostics.rs` | B |
| 314 | `scripts/chs_parity.sh` | C |
| 298 | `src/bench.rs` | B |

**Already ≤256 (oodac):** `lex.oo`, `token_emit.oo`, `token_fmt.oo`, `check_caps.oo`.

---

*Product architecture: `DESIGN.md`. Beta: `BETA.md`. Process lenses: monorepo `RULES.md` / `OODA.md`. Line Lock: `scripts/check_file_lines.sh`.*
