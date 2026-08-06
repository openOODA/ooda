# `for` / `match` on Backend-C

| Construct | Lex | Check | Emit (Backend-C) |
|-----------|-----|-------|------------------|
| `for i in LO..HI { … }` **INT** bounds | `KW_FOR` | loop var bound | **Lowered** → `for (long long i = lo; i < hi; i++)` |
| `for i in lo..hi` non-INT bounds | `KW_FOR` | OK if names known | **`ERR\tc_emit\tfor residual …`** fail-closed |
| `match` **stmt** on `Result` `Ok`/`Err` | `KW_MATCH` | as today | **Lowered** → `if (__m.ok) { … } else { … }` |
| `let x = match r { Ok/Err … }` | after `=` | Result-shaped | **Lowered** — arm payload binds + arm exprs |
| Incomplete match (missing Err) | `KW_MATCH` | — | **`ERR\tc_emit\tmatch: expected Err(...) arm`** |
| `while` / `if` / `else` | KW_* | OK | **Lowered** |

**Not claimed:** Option match, expr bounds for `for`, `..=`, break/continue beyond C `for`, full pattern exhaustiveness beyond Ok/Err, match on non-Result.

## Rails

- Pass for: `bootstrap/corpus/emit-c/pass/for_range_int.oo`
- Fail for (non-INT bounds): `bootstrap/corpus/emit-c/fail/for_range_residual.oo`
- Pass match stmt: `bootstrap/corpus/emit-c/pass/match_result_stmt.oo`
- Pass match-let: `bootstrap/corpus/emit-c/pass/match_result_let.oo`
- Fail incomplete match: `bootstrap/corpus/emit-c/fail/match_incomplete.oo`
- Typecheck pass: `bootstrap/corpus/typecheck/pass/for_range_names.oo`
- Emit: `oodac/c_emit_match.oo`, `oodac/c_emit_stmt.oo`; lex: `token_emit.oo` (`match`→`KW_MATCH`)
- Host-era demo only: `fixtures/for_range.oo`

## Result `.unwrap()`

| Form | Emit | Runtime |
|------|------|---------|
| `r.unwrap()` on Ok | ternary → `.val` | returns payload |
| `r.unwrap()` on Err | same | prints `ERR\tunwrap` + `process_exit(1)` |

- Pass: `bootstrap/corpus/emit-c/pass/result_unwrap_ok.oo`, `fixtures/result_unwrap.oo`
- Runtime fail demo: `fixtures/result_unwrap_err.oo` (emit OK; binary exits 1)

## Workarounds (only when residual applies)

- Non-INT range → `while` + counter
- Option match → still residual; use `is_some` / `is_none` if lowered, else fail-closed
