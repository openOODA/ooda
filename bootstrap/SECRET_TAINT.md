# Static taint (`// SECRET:`) — product surface + residual

**Marker:** `SECRET_TAINT_RESIDUAL_ALPHA`  
**Status:** path A–C + write_file sink **In**. PM **3.5**.  
**Sprint:** M52–M60, M113, M128 (sink), M127 honesty re-sync.

## Product surface (In)

| Form | Intent | Status |
|------|--------|--------|
| `// SECRET: name` | Line-start tags name as secret | **In** |
| `println(secret)` bare IDENT | Sink refuse emit + check | **In** |
| Assign-prop `let y = x` / chains | Tag copy multi-hop | **In** |
| Concat / call-arg / call-return (same file) | RHS IDENT prop → println refuse | **In** (M113) |
| Multi-arg `println(a, secret)` | Refuse secret arg | **In** |
| `write_file(fs, path, secret)` content IDENT | Non-println sink refuse | **In** (M128) |
| LLVM `emit-llvm` | Same check dual-path before IR | **In** (M129) |
| `#[Secret]` attribute | DESIGN grammar | **residual** |

## What is true today

| Layer | Behavior |
|-------|----------|
| **Parse / check** | No `#[Secret]` grammar; `oodac check` dual-path for println + write_file + assign-prop |
| **Backend-C emit** | File-level `// SECRET:` tags; refuse secret bare IDENTs at println and write_file content |
| **Interproc (same TU)** | Call-arg / return / concat IDENT prop into sink refuse |
| **Other sinks** | **NetCap / fetch** URL / log / remaining OS sinks residual |
| **Honesty** | Not full IFC; not attribute grammar; sticky tag residual |

**Fail-closed residual:** do **not** treat this as a full confidentiality boundary or “taint tracking shipped” for all sinks.

## What we do **not** claim

- Interprocedural full-program alias analysis across modules  
- Secret→NetCap / fetch / arbitrary log sinks as product green  
- Full IFC / “taint tracking shipped/enforced” for the whole language  
- `#[Secret]` attribute enforcement  
- Cryptographic redaction or runtime scrubbing  

## Rails

- Doc marker: `SECRET_TAINT_RESIDUAL_ALPHA`
- Residual smoke: `scripts/secret_taint_residual_smoke.sh`
- Enforce smoke: `scripts/secret_sink_enforce_smoke.sh`
- Fixtures: `secret_sink_*`, `secret_assign_*`, `secret_chain_fail`, `secret_concat_fail`, `secret_call_*`, `secret_write_file_*`, …
