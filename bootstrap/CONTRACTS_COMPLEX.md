# Complex contracts residual
**Status:** residual honesty. PM **1.2**. **Marker:** `CONTRACTS_COMPLEX_RESIDUAL_ALPHA`
## Named / partial surface
- Simple `requires IDENT OP lit|ident` + simple `ensures result OP lit|ident` runtime In (M9/M19)
- **Multi-clause simple AND In (M51):** multiple simple requires/ensures on one fn → sequential runtime checks (ensures cap 8)
- Rails: `fixtures/multi_clause_pass.oo`, `multi_clause_{req,ens}_fail.oo`, `scripts/contracts_multi_clause_smoke.sh`
## Path A product floor — simple `&&` (M159)

**In:** simple comparison clauses combined with `&&` / `||` (M162) in `requires` / `ensures` (runtime checks via emit).  
**Rails:** `scripts/contracts_and_smoke.sh` + multi-clause smokes.  
**Fixtures:** `complex_contract_{pass,req_fail,ens_fail}.oo`

## Fail-closed residual
Still residual: arithmetic-heavy exprs, SMT, quantifiers, old-state, full contract language, >8 ensures.
Do not treat residual gaps as DESIGN-complete.
## What we do **not** claim
Full DESIGN depth (SMT, quantifiers, old-state, multi-expr non-simple clauses) is not product alpha.

## Rails
- `CONTRACTS_COMPLEX_RESIDUAL_ALPHA`
- `scripts/contracts_complex_residual_smoke.sh`
