# Path A floating-point math & trigonometry — product floor + honesty

**Status:** Path A product free names **In** (alpha).  
**Marker:** `MATH_TRIG_PATH_A_ALPHA`  
**Mission:** M166 — IEEE-754 **double** trig/exp floor; no decimal type claim.

## Path A free names (In)

| Free name | Result | Notes |
|-----------|--------|-------|
| `sin(x)` | `Float` | radians; → `oo_sin` / `math.h sin` |
| `cos(x)` | `Float` | radians; → `oo_cos` |
| `ln(x)` | `Float` | natural log; → `oo_ln` / `log` |
| `exp(x)` | `Float` | → `oo_exp` |
| `sqrt(x)` | `Float` | → `oo_sqrt` |
| `pow(base, exp)` | `Float` | → `oo_pow` |

**Types:** product `Float` is an **IEEE-754 double** alias (Backend-C: `double`).  
`f64` annotation also maps to `double`. **No** decimal / BigDecimal / soft-float type.

## Runtime

| Symbol | TU |
|--------|-----|
| `oo_sin` / `oo_cos` / `oo_ln` / `oo_exp` / `oo_sqrt` / `oo_pow` | `runtime/chs_rt_math.c` |
| `oo_print_double` | `chs_rt_math.c` (`printf %g`) |

Umbrella: `#include "chs_rt_math.c"` from `chs_rt.c`. Link already uses `-lm`.

## std product wrappers

`std/math.oo`: `math_sin` / `math_cos` / `math_ln` / `math_exp` / `math_sqrt` / `math_pow`  
plus docs-only `type Float64 = Float`.

## What is true today

| Layer | Behavior |
|-------|----------|
| **Lex** | `FLOAT` tokens in `token_scan_number.oo` |
| **Types** | `Float` / binops / compares already in `tc_*` |
| **Emit** | free names lower via `c_emit_libfloor`; `Float` → `double` in `c_ty_at` |
| **Print** | `println` of double → `oo_print_double` (`%g`) |
| **Semantics** | host `libm` IEEE double; domain errors = NaN/inf (not Result) |

## Residual honesty (do **not** claim)

- Decimal / fixed-point / `BigDecimal` product type
- Soft-float or cross-platform bit-identical results beyond host IEEE
- Result-typed domain errors for ln/sqrt of negative
- Full complex / hyperbolic / special-function library
- WASM/LLVM full parity of every free name (Backend-C path A first)

## Rails

- Doc marker: `MATH_TRIG_PATH_A_ALPHA`
- Smoke: `scripts/math_trig_smoke.sh`
- Fixture: `fixtures/math_trig.oo`
- Wiring: `tc_names`, `tc_call_arity`, `c_emit_libfloor`, `c_emit_preamble`, `c_emit_ty`, `c_emit_print`
