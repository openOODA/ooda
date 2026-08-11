# Complex contracts residual
**Status:** residual honesty. PM **1.2**. **Marker:** `CONTRACTS_COMPLEX_RESIDUAL_ALPHA`
## Named / partial surface
- Simple `requires IDENT OP lit|ident` + simple `ensures result OP lit|ident` runtime In (M9/M19)
- **Multi-clause simple AND In (M51):** multiple simple requires/ensures on one fn → sequential runtime checks (ensures cap 8)
- Rails: `fixtures/multi_clause_pass.oo`, `multi_clause_{req,ens}_fail.oo`, `scripts/contracts_multi_clause_smoke.sh`
## Fail-closed residual
Complex forms stay fail-closed at emit: `&&` / arithmetic exprs / SMT / full contract language / >8 ensures.
Do not treat residual gaps as DESIGN-complete.
## What we do **not** claim
Full DESIGN depth (SMT, quantifiers, old-state, multi-expr clauses) is not product alpha.

## Path A product floor (alpha) — M153

**Path A marker:** `CONTRACTS_COMPLEX_PATH_A_ALPHA`  
**Status:** path A **In** — check default-deny of named residual free calls (`check_residual.oo`).  
**In:** simple+multi-clause AND In; &&/SMT still residual fail-closed  
**Rails:** `scripts/residual_path_a_floor_smoke.sh`  
**Still residual:** full DESIGN implementation of this moonshot (not claimed).

## Rails
- `CONTRACTS_COMPLEX_RESIDUAL_ALPHA`
- `scripts/contracts_complex_residual_smoke.sh`
