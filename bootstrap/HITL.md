# Human-in-the-loop (`hitl`) testing — path A deny-mode + residual interactive

**Marker residual:** `HITL_RESIDUAL_ALPHA`  
**Path A marker:** `HITL_PATH_A_ALPHA`  
**Status:** path A **In** (M157 non-interactive deny-mode + residual free-name refuse). Interactive harness residual. PM **5.6**.

## Product surface

| Form | Behavior (alpha) |
|------|------------------|
| `// HITL: pause` | **In (M157):** line-start marker → check fail-closed `ERR\thitl\t…` / `--json-errors` **E_HITL** (CI / non-interactive deny-mode) |
| `verify_human("…")` | **Residual free-name refuse** at check (`E_RESIDUAL`) — not a product harness call |

## What is true today

| Layer | Behavior |
|-------|----------|
| **Check** | `// HITL: pause` denied (non-interactive product path); `verify_human` free call refused |
| **Runtime / CLI** | **No** interactive HITL harness (TTY prompt / record / replay / approve) |
| **Agent / product** | **Not** agent pause/resume product |
| **Honesty** | This file + `hitl_product_floor_smoke` + `hitl_residual_smoke` |

**Fail-closed residual:** deny-mode is **not** human attestation. It only blocks silent green when the pause marker is present. Full DESIGN `verify_human` approval loops remain residual.

## What we do **not** claim

- Interactive HITL harness (TTY prompt / record / replay / deny / skip modes)  
- “HITL fully shipped” as interactive product green  
- Agent pause/resume product surface  
- Capability-gated human attestation (`&HumanCap`-class)  
- Full DESIGN `verify_human` CLI approval before marking a build passing  

## Path A product floor (alpha) — M157

**In:**  
- `// HITL: pause` → non-interactive deny at check (`E_HITL`)  
- `verify_human` free-name refuse (M153 residual free-name path A)  

**Rails:**  
- `scripts/hitl_product_floor_smoke.sh`  
- `scripts/hitl_residual_smoke.sh`  
- Fixtures: `hitl_pause_fail.oo`, `hitl_pause_pass.oo`, `hitl_marker.oo` (docs/marker rail)

## Residual next (not this floor)

Interactive harness; agent pause/resume; live `verify_human` product semantics.
