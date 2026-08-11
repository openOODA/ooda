# AI-native systems language — product floor (alpha)

**Marker:** `AI_NATIVE_PATH_A_ALPHA`  
**PM:** executive **AI-native systems language** + tooling **2.1 / 2.2 / 2.2b / 5.5**  
**Status:** path A **In** — agent loop + bounded `ooda fix` (E_CAP + E_TC + E_HITL); full DESIGN AI stack residual.

## What is production-ready (alpha)

| Surface | Behavior | Rail |
|---------|----------|------|
| `ooda outline` | Token-cheap pub fn list (parse-only) | `outline_reflect_smoke` |
| `ooda reflect` | NDJSON symbol / caps / contracts / verify | same |
| `ooda check --json-errors` | Machine codes + `fix_hint`; **E_CAP** also `kind` + `suggested_fix` | `json_errors_smoke` |
| `ooda patch … --replace-fn` | Surgical function body replace (path-safe) | `patch_smoke` |
| `ooda fix` | **E_CAP** (M155), **E_TC undefined-var** (M158), **E_HITL** exact pause remove (M165) | `ecap`/`etc`/`ehitl` autofix smokes |
| **Agent loop floor** | outline → reflect → json-errors → patch in one smoke | `ai_native_product_floor_smoke` |
| Residual free names | telepathic/hive free calls refuse | `residual_path_a_floor_smoke` |

## What we do **not** claim

- Full multi-code AST auto-apply (only E_CAP + E_TC-undefined + E_HITL classes) — see `AST_AUTOFIX.md`  
- Free-form `suggested_fix` eval / shell-eval of diagnostics  
- Intent-driven / telepathic compile (`TELEPATHIC_AST.md`)  
- Global hive-mind fuzzing (`HIVEMIND.md`)  
- Typed import-graph outline depth  

## Rails

- `scripts/ai_native_product_floor_smoke.sh`  
- `scripts/ecap_autofix_smoke.sh`  
- `scripts/etc_autofix_smoke.sh`  
- `scripts/ehitl_autofix_smoke.sh`  
- Component: outline/reflect, patch, json-errors  
- Residual: `ast_autofix_residual_smoke`, telepathic/hive residual packs  

