# AST auto-apply residual
**Status:** residual honesty. PM **2.1**. **Marker:** `AST_AUTOFIX_RESIDUAL_ALPHA`
## Named / partial surface
See PM notes for what is already product In.
## Fail-closed residual
Do not treat residual gaps as DESIGN-complete.
## What we do **not** claim
Full DESIGN depth for this item is not product alpha.

## Path A product floor (alpha) — M153 + M154 agent loop

**Path A marker:** `AST_AUTOFIX_PATH_A_ALPHA`  
**Status:** path A **In** for **hints**, not full auto-apply.  
**In:**  
- `--json-errors` `fix_hint` on codes (E_CAP/E_TC/E_PARSE/E_SECRET/…)  
- **E_CAP** also ships machine `kind` + `suggested_fix` for agent apply guidance  
- Full agent loop floor: `scripts/ai_native_product_floor_smoke.sh`  
**In (M155):** `ooda fix <file.oo>` / `scripts/ooda_apply_ecap_fix.sh` — bounded E_CAP structural apply (add `&Cap` param + first-arg token); rails `ecap_autofix_smoke`.
**Still residual:** auto-apply for non-E_CAP codes; full AST rewrite; telepathic compile.

## Rails
- `AST_AUTOFIX_RESIDUAL_ALPHA`
- `scripts/ast_autofix_residual_smoke.sh`
