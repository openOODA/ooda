# openOODA v0.117.0-alpha
Shipper: **Grok 4.5 (xAI)** — loop 1/30 R1 typecheck lit-binop expand.

## Top-5
1. oodac silent OK on `1 + "hi"` (honesty D↑)
2. Pure lit binop type combine in `.oo` (`infer_pure_lit_expr_type`)
3. Annotated init chains `let x: Int = 1 + "a"` fail-closed
4. Free expr-stmt lit binop fail-closed
5. Corpus + cargo + chs_parity; fix unused `match_idx` warning (W/D)

## E-M
D↓ false green typecheck; W↓ token walk only; V↑ R1 self-host surface.

## Scoreboard
RS_COUNT still stage-0 (no .rs deleted). Not full typecheck/beta.

## Pin
v0.117.0-alpha
