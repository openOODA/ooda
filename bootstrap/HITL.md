# M24 Human-in-the-loop (`hitl`) testing — residual at alpha

**Marker:** `HITL_RESIDUAL_ALPHA`  
**Status:** residual honesty (not enforced). PM **5.6** / sprint **M24**.

## Product surface (names only)

| Form | Intent |
|------|--------|
| `verify_human("…")` | DESIGN primitive (subjective approval in test loops) |
| `// HITL: pause` | Simpler product marker (comment form) |

Either form **names** a HITL pause point. At alpha neither opens an interactive harness nor pauses agent/product execution.

## What is true today

| Layer | Behavior |
|-------|----------|
| **Parse / check** | No dedicated `verify_human` / hitl grammar; comment marker is source-level only |
| **Runtime / CLI** | **No** interactive HITL harness shipped; no TTY prompt / record / replay modes |
| **Agent / product** | **Not** agent pause/resume product; no CI gate on human approval |
| **Honesty** | Residual documented here + `scripts/hitl_residual_smoke.sh` |

**Fail-closed residual:** do not treat presence of `// HITL: pause` or `verify_human(…)` as a human-attestation boundary. Autonomous loops are **not** required to pause for human review by the product.

## What we do **not** claim

- Interactive HITL harness (TTY prompt / record / replay / deny / skip modes)  
- “HITL shipped” / “HITL enforced” as product green  
- Agent pause/resume product surface  
- Capability-gated human attestation (`&HumanCap`-class)  
- Full DESIGN `verify_human` CLI approval before marking a build passing

## Rails

- Doc marker: this file must contain `HITL_RESIDUAL_ALPHA`
- Smoke: `scripts/hitl_residual_smoke.sh` (wired in `ci_product`)
- Fixture (marker only, not enforced): `fixtures/hitl_marker.oo`

## Next (not this sprint)

Path A candidate: parse `// HITL: pause` or `verify_human(…)` and fail-closed (deny mode) in non-interactive CI only — still **not** interactive harness or agent pause/resume.
