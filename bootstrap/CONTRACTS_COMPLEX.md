# Complex contracts residual
**Status:** residual honesty. PM **1.2**. **Marker:** `CONTRACTS_COMPLEX_RESIDUAL_ALPHA`
## Named / partial surface
- Simple `requires IDENT OP lit|ident` + simple `ensures result OP lit|ident` runtime In (M9/M19)
- **Multi-clause simple AND In (M51):** multiple simple requires/ensures on one fn → sequential runtime checks (ensures cap 8)
- Rails: `fixtures/multi_clause_pass.oo`, `multi_clause_{req,ens}_fail.oo`, `scripts/contracts_multi_clause_smoke.sh`
## Path A product floor — simple `&&` / `||` (M159/M162)

**In:** simple comparison clauses combined with `&&` / `||` in `requires` / `ensures` (runtime checks via emit).  
**Rails:** `scripts/contracts_and_smoke.sh` + multi-clause smokes.  
**Fixtures:** `complex_contract_{pass,req_fail,ens_fail}.oo`, `contract_or_{pass,fail}.oo`

## Path A product floor — simple arith + compare (M165)

**In:** simple arithmetic in contract exprs lowered by `c_emit_contract.oo` (`+ - * / %`) with compares and paren/`&&`/`||`, e.g.  
- `requires x + 1 > 0`  
- `requires (a > 0 || b > 0) && c >= 0`  
Runtime assert only (not SMT solve).  
**Rails:** `scripts/contracts_arith_smoke.sh`  
**Fixtures:** `contract_arith_pass.oo`, `contract_arith_req_fail.oo`, `contract_arith_or_and_fail.oo`

## Fail-closed residual
Still residual: **full SMT**, **quantifiers (`forall` / `exists`)**, **old-state**, heavy multi-expr algebra beyond simple arith+compare, full contract language, >8 ensures.  
Do not treat residual gaps as DESIGN-complete.
## What we do **not** claim
Full DESIGN depth (SMT solver, quantifiers, old-state, arbitrary multi-expr non-simple clauses) is not product alpha. **No quantifiers. No old-state.** Path A is runtime emit of simple/arith/logic clauses only.

## Rails
- `CONTRACTS_COMPLEX_RESIDUAL_ALPHA`
- `scripts/contracts_complex_residual_smoke.sh`
- `scripts/contracts_arith_smoke.sh`
