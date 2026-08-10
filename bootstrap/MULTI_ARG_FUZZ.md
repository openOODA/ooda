# Multi-arg pure fuzz (M49 Int + M56 Bool + M106 String + residual)

**Status:** Int **arity-2/3 In**; Bool **arity-2 In**; String **arity-2 In (M106)**; arity≥4 / multi List / Bool|String arity≥3 residual. PM **3.6**.  
**Marker:** `MULTI_ARG_FUZZ_RESIDUAL_ALPHA` (arity≥4 residual honesty)

## Named / partial surface

- Pure fuzz domains Int/Bool/String/List shipped
- **Int arity-2 multi-arg In:** samples `__fuzz_x` + `__fuzz_y`, calls `f(x,y)`, rewrites `x`/`y`/`result`
- **Int arity-3 multi-arg In:** samples `__fuzz_x`/`__fuzz_y`/`__fuzz_z`, calls `f(x,y,z)`, rewrites `x`/`y`/`z`/`result`
- **Bool arity-2 multi-arg In (M56):** samples two Bools, calls `f(x,y)`, rewrites `x`/`y`/`result`
- **String arity-2 multi-arg In (M106):** samples two Strings (len range from target), calls `f(x,y)`, rewrites `x`/`y`/`result`
- Arity from target `fn` signature; multi-Int all `Int`; multi-Bool all `Bool`; multi-String all `String`
- Single-arg domains unchanged

## Fail-closed residual

- **Arity ≥4** pure path fail-closed (`arity>=4 fail-closed`)
- **Bool multi arity≥3** / **String multi arity≥3** fail-closed
- **Multi-arg List** domain fail-closed (`multi-arg non-int`)
- Do **not** treat partial product depth as DESIGN-complete multi-param fuzzer

## What we do **not** claim

- No arity≥4 pure multi-arg fuzzer shipped
- No multi-arg pure fuzzer for List
- No Bool/String multi arity≥3 pure path
- No AST-parsed contracts (marker rewrite of `x`/`y`/`z`/`result` only)

## Rails

- Doc marker: `MULTI_ARG_FUZZ_RESIDUAL_ALPHA`
- Smoke (In): `fuzz_multi_arg_smoke.sh` — Int 2/3, Bool 2, String 2, residual arity4 / weak / list multi
- Smoke (residual honesty): `multi_arg_fuzz_residual_smoke.sh`
- Fixture residual marker: `fixtures/multi_arg_fuzz_marker.oo`
