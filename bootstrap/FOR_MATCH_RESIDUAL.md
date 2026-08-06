# `for` / `match` on Backend-C — honest residual

**Status:** fail-closed residual. Prefer freeze + fixtures over half-broken lower.

| Construct | Lex | Check | Emit (Backend-C) |
|-----------|-----|-------|------------------|
| `for i in lo..hi { … }` | `KW_FOR` | names bind (`i` in body) | **`ERR\tc_emit\tfor residual …`** + exit 1 |
| `match scrut { … }` as **stmt** | `KW_MATCH` | arms / binds as today | **`ERR\tc_emit\tmatch residual …`** + exit 1 |
| `let x = match r { Ok/Err … }` | `KW_MATCH` after `=` | OK for Result-shaped | Partial: binds `x = scrut.val` (no arm eval) |
| `while cond { … }` | `KW_WHILE` | OK | **Lowered** (product path) |
| `if` / `else` | `KW_IF` / `KW_ELSE` | OK | **Lowered** (product path) |

**Not claimed:** full range-for desugar, full match/pattern lowering, match exhaustiveness at emit.

## Rails

- Fail fixtures: `bootstrap/corpus/emit-c/fail/for_range_residual.oo`, `match_stmt_residual.oo`
- Smoke: `scripts/c_emit_smoke.sh` globs `emit-c/fail/*.oo` → must see `ERR\tc_emit` or non-zero
- Emit site: `oodac/c_emit_stmt.oo` (`KW_FOR` / `KW_MATCH` branches)
- Check: `oodac/tc_names.oo` binds loop var after `KW_FOR` (so residual is emit, not a fake undefined-name)
- Historical WASM/host demos: `fixtures/for_range.oo` (+ `.wat`) — **not** a Backend-C product claim (`fixtures/README.md`)
- Typecheck-only: `bootstrap/corpus/typecheck/pass/for_range_names.oo`, `match_bind_ok.oo`

## Why freeze (for now)

Range-for *could* desugar to:

```text
let mut i = lo;
while i < hi {
  /* body */
  i = i + 1;
}
```

That is only safe when:

1. `lo` / `hi` are pure int expressions (no double-eval surprises),
2. loop bind mutability/shadowing matches DESIGN,
3. `break` / `continue` / nested for are defined,
4. pass **and** fail rails land before any product claim.

A half-emit that drops the body or emits wrong C is worse than residual. **Ship residual.**

Statement `match` needs arm selection, binds, and Result/Option encoding — larger than a drive-by. Use `is_ok` / `is_err` + payload until a real lower lands.

## When to implement

### Range-for (MVP)

Implement only when **all** hold:

1. Pure path; module stays ≤250 lines (split if needed)
2. Desugar = int range only (`IDENT in expr DOTDOT expr` or lit bounds) → while + counter
3. Pass fixture (sum / println loop) **and** keep residual fail for non-range `for` if any
4. `c_emit_smoke` green; no silent skip of `KW_FOR`
5. Document residual: non-int ranges, iterators, patterns still out

### Match stmt (MVP)

Implement only when **all** hold:

1. Result (and/or Option) arm lower with real branch, not only `.val` bind
2. Fail-closed on unsupported patterns (not silent first-arm)
3. Pass + fail rails; no claim of full algebraic match
4. Align with `std/result.oo` / `std/option.oo` honesty notes

## Interim agent guidance

- Prefer **`while`** + counter over `for i in a..b` on pure product / emit-c
- Prefer **`if x.is_ok()` / `is_err()`** (and payload field) over statement `match`
- Treat `ERR\tc_emit\tfor residual` / `match residual` as **intentional honesty**, not a flaky smoke
- Do not reintroduce WASM/LLVM product paths to “support for”; see `bootstrap/P4_DROPS.md`
- Do not add `emit-c/pass/for_range_*.oo` until range-for is actually lowered

## Related

- `oodac/c_emit_stmt.oo` — residual branches
- `oodac/c_emit_let.oo` — partial `let = match` Result `.val` path
- `bootstrap/FLOOR.md`, `bootstrap/BUILD_OUT.md`, `bootstrap/P4_DROPS.md`
