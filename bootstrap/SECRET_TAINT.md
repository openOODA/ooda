# Static taint (`// SECRET:`) — product surface + residual

**Marker:** `SECRET_TAINT_RESIDUAL_ALPHA`  
**Status:** println + write_file + fetch URL + **sys_exec argv** sinks **In**. PM **3.5**.  
**Sprint:** M52–M60, M113, M128–M131, **M135 (sys_exec)**.

## Product surface (In)

| Form | Intent | Status |
|------|--------|--------|
| `// SECRET: name` | Line-start tags name as secret | **In** |
| `println(secret)` bare IDENT | Sink refuse emit + check | **In** |
| Assign-prop / concat / call prop | Tag copy multi-hop | **In** |
| `write_file(..., secret)` content | Refuse | **In** (M128) |
| `fetch(net, secret)` URL | Refuse | **In** (M131) |
| `sys_exec(sys, …, secret)` argv | Refuse bare SECRET IDENT after cap | **In** (M135) |
| LLVM `emit-llvm` dual-path | Same checks before IR | **In** (M129) |
| `#[Secret]` attribute | DESIGN grammar | **residual** |

## What is true today

| Layer | Behavior |
|-------|----------|
| **Check + emit** | Refuse secret bare IDENTs at listed sinks |
| **Other sinks** | Remaining OS / log / NetCap friends residual |
| **Honesty** | Not full IFC; not attribute grammar |

**Fail-closed residual:** not a full confidentiality boundary for all sinks.

## What we do **not** claim

- Full-program IFC / all OS sinks  
- `#[Secret]` attribute enforcement  
- Cryptographic redaction  

## Rails

- Marker: `SECRET_TAINT_RESIDUAL_ALPHA`
- Enforce: `scripts/secret_sink_enforce_smoke.sh`
- Residual: `scripts/secret_taint_residual_smoke.sh`
- Fixtures: `secret_*`, including `secret_sys_exec_*`
