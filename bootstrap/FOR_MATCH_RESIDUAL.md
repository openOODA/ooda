# `for` / `match` on Backend-C

| Construct | Lex | Check | Emit (Backend-C) |
|-----------|-----|-------|------------------|
| `for i in LO..HI { … }` **INT** bounds | `KW_FOR` | loop var bound | **Lowered** → `for (long long i = lo; i < hi; i++)` |
| `for i in lo..hi` non-INT bounds | `KW_FOR` | OK if names known | **`ERR\tc_emit\tfor residual …`** fail-closed |
| `match` as **stmt** | `KW_MATCH` | as today | **`ERR\tc_emit\tmatch residual …`** |
| `let x = match r { Ok/Err … }` | after `=` | Result-shaped | Partial: `x = scrut.val` |
| `while` / `if` / `else` | KW_* | OK | **Lowered** |

**Not claimed:** expr bounds, `..=`, break/continue beyond C `for`, full match patterns.

## Rails

- Pass: `bootstrap/corpus/emit-c/pass/for_range_int.oo`
- Fail (non-INT bounds): `bootstrap/corpus/emit-c/fail/for_range_residual.oo`
- Fail match stmt: `bootstrap/corpus/emit-c/fail/match_stmt_residual.oo` (if present)
- Typecheck pass: `bootstrap/corpus/typecheck/pass/for_range_names.oo`
- Emit: `oodac/c_emit_stmt.oo`; lex: `token_emit.oo` (`for`→`KW_FOR`); names: `tc_names.oo` binds after `KW_FOR`
- Host-era demo only: `fixtures/for_range.oo`

## Workarounds

- Non-INT range → `while` + counter
- Match stmt → `if is_ok` / `is_err`
