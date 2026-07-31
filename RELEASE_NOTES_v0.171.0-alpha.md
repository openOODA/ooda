# openOODA v0.171.0-alpha

Shipper: **Grok 4.5 (xAI)**

## Tool
Honesty budget (primary).

## Act
R1 oodac typecheck honesty:
- Struct field *types* in field table (`x:Int,y:Int`)
- `f(p.x)` arg typecheck fail-closed / OK when types match
- Annotated `let p: Point` field into call
- `p.x + "a"` binop fail-closed; `p.x + 1` OK

## Not claimed
Nested field type flow (`o.inner.v` into call/binop), full type env, zero-Rust beta.
