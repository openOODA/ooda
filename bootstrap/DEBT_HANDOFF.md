# Technical debt handoff (ready for scan)

**Tip context:** after depth commit on `main` (nested blocks, `%`/MOD, free-prep softeners, M6×11, M3 pure Int fixtures).  
**Gate:** local `ci_product` green with `source ~/.local/ooda-toolenv/env.sh`.  
**Not beta.** Owner freeze only.

## Proven green (do not re-litigate without re-run)
- `arc_smoke` — 4 fixtures (incl. nested_scope_str, list push/get)
- `bc_vm_smoke` — 11 fixtures interpreter (**not JIT**), includes `%` → MOD
- `ci_product` — full rail when wasmtime+clang on PATH
- Free is **leak-safe** (honest residual)

## High-value debt (scan first)

### M2 / memory
1. **Real free** still UAF under seed pure multi — need tree-owned pure multi or full seed ownership audit
2. Softeners are **regex** (`pure_rewrite_formals`, `pure_rewrite_alias_retain`) — incomplete
3. Nested bare-block **shadowing** not smoke-proved
4. List free (elements + header) not on reclaim path

### Typecheck / lex parity
5. `% 0` not fail-closed like `/ 0` (if DIV0 exists)
6. LLVM: multi-binop + `%` (srem) **landed** (see `llvm_execute_smoke`); WASM PERCENT residual if unclaimed
7. `lex.oo` near **MAX_LINES=256** (252) — next lex tweak needs split

### Emit hosts
8. Seed still pure-multi emit host for oodac
9. Tree emit intermittent on large graphs historically
10. `c_emit` early_return seed path can mis-lower println String → print_int (seed residual)

### M3 fuzz
11. Pure Int only — multi-type pure fuzz still fail-closed
12. Verify-without-fuzz may still use Python residual paths

### M4/M5/M6 surface
13. M5 multi-binop IR holes — **closed** (Pratt prec + `%`→srem; smoke proves `2+3*4` and `10%3`)
14. M6: match/struct/list/string methods not on VM smoke
15. Product `ooda run` may Backend-C not always BC VM

### Process / hygiene
16. Host toolenv is **local** (`~/.local/ooda-toolenv`) — CI matrix must install wasmtime/clang or fail closed (already does)
17. Monorepo SPRINT.md must track tip SHA after each push
18. Avoid concurrent pure_build + ci_product (race deletes `oodac/oodac`)
19. **T4 bak ignore:** `oodac/oodac.bak*`, `oodac_new`, emit temps gitignored — do not commit; local `rm` optional (see `AUDIT_RESIDUAL.md` §T4)
20. **T4 minisign tool path:** no vendored `tools/minisign`; PATH or operator-local binary or loud `OODA_SEED_ALLOW_UNSIGNED` (`seed/SIGNING.oot`)
21. **T4 monofile pressure:** `chs_rt_ffi.c` >350 (359); peel residual — not a silent Lock green
22. **8.1 DEBUG:** live `tc_control_cond.oo` has product `ERR\ttype` diagnostics only — not a DEBUG leak residual until re-found

### M21/M48/M54 MaxCycles (while + range-for fuel In; residual remains)
23. Path A/B: file-level `// MAX_CYCLES: N` → Backend-C `while` + INT..INT `for` body fuel (`ERR\tmax_cycles\texceeded`); still residual: OS cgroup, recursion / non-range for, `#[MaxCycles]` attribute — see `MAX_CYCLES.md`

### M22/M52–M55 Static taint (path A/B In; residual remains)
24. **In:** line-start `// SECRET: name` → bare `println(ident)` refuse (emit + check dual-path) + direct IDENT assign-prop; **residual:** interproc, concat/call taint, NetCap/non-println sinks, `#[Secret]` attr — see `SECRET_TAINT.md`

## Softeners inventory (do not delete without free plan)
- `scripts/pure_rewrite_formals.py`
- `scripts/pure_rewrite_alias_retain.py`
- `scripts/oodac_pure_rewrite.py` **retired** (native `c_emit` grants caps; file → `.retired`)

## Suggested debt-scan order
1. M2 free ownership graph (seed emit vs tree)
2. Line-budget splits (`lex.oo`, fat emit modules)
3. Typecheck fail corpus for `%` and zero
4. Backend parity (LLVM/WASM vs C/BC)
5. Fuzz multi-type pure domain design (no soft-pass)

## Honesty rails
```bash
source ~/.local/ooda-toolenv/env.sh
cd ooda && ./scripts/ci_product.sh
./scripts/residual_honesty_smoke.sh
./scripts/arc_smoke.sh
./scripts/bc_vm_smoke.sh
```
