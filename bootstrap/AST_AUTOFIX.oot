# AST auto-apply — path A floors + residual multi-code depth

**Marker residual:** `AST_AUTOFIX_RESIDUAL_ALPHA`  
**Path A marker:** `AST_AUTOFIX_PATH_A_ALPHA`  
**PM:** **2.1**. Status: bounded product apply **In** for named classes; full multi-code AST rewrite residual.

## What is production-ready (alpha)

| Class | Behavior | Rails |
|-------|----------|-------|
| **Hints** | `--json-errors` `fix_hint` (E_CAP/E_TC/E_PARSE/E_SECRET/…); E_CAP also `kind` + `suggested_fix` | `json_errors_smoke` |
| **E_CAP apply (M155)** | `ooda fix` / `ooda_apply_ecap_fix.py` — structural add `&Cap` param + first-arg token | `ecap_autofix_smoke` |
| **E_TC undefined-var (M158)** | `ooda fix` / `ooda_apply_etc_fix.py` — insert `let name = 0;` in enclosing fn body | `etc_autofix_smoke` |
| **E_HITL pause (M165)** | `ooda fix` / `ooda_apply_ehitl_fix.py` — remove lines that strip to exact `// HITL: pause` | `ehitl_autofix_smoke` |
| **Agent loop** | outline → reflect → json-errors → patch | `ai_native_product_floor_smoke` |
| **Dispatcher** | `ooda_apply_fix.py` multi-pass: E_CAP → E_TC → E_HITL | product `ooda fix` |

## What we do **not** claim

- Auto-apply for **other** diagnostic codes (E_PARSE brace rewrite, multi-error batches, E_SECRET, …)  
- Full AST rewrite / apply of free-form `suggested_fix` text (never shell-eval of diagnostics)  
- Free-form rewrite of arbitrary comments (E_HITL only exact `// HITL: pause` lines)  
- Telepathic / intent compile  

## Fail-closed residual

Non-applicable inputs (e.g. parse-only fail, no E_CAP/E_TC-undefined/E_HITL) exit non-zero. Do not treat residual multi-code depth as DESIGN-complete.

## Rails

- `AST_AUTOFIX_RESIDUAL_ALPHA` / `AST_AUTOFIX_PATH_A_ALPHA`  
- `scripts/ecap_autofix_smoke.sh`  
- `scripts/etc_autofix_smoke.sh`  
- `scripts/ehitl_autofix_smoke.sh`  
- `scripts/ast_autofix_residual_smoke.sh`  
