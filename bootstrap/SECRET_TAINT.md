# Static taint (`// SECRET:`) — path A product floor (alpha) + residual

**Marker:** `SECRET_TAINT_RESIDUAL_ALPHA`  
**Status:** **Path A product floor In (alpha).** PM **3.5** → **done (alpha)** for path A.  
**Residual listed:** `#[Secret]` attr, full IFC, every OS/log sink, crypto redaction, NetCap friends beyond the table — not soft-pass.

## Product surface (In — path A floor alpha)

| Form | Status |
|------|--------|
| `// SECRET: name` tag | **In** |
| path A bare IDENT **println/eprintln** refuse | **In** |
| Assign / concat / call prop (same file) | **In** |
| `write_file` content + path | **In** |
| `fetch` URL | **In** |
| `sys_exec` argv | **In** |
| `env_get` key | **In** |
| `read_file` / `path_exists` / `file_size` path | **In** |
| `seed` / `sleep_ms` / `alloc_bytes` / `free_bytes` / `malloc` / `free` / `realloc` values | **In** |
| `process_exit` code | **In** |
| LLVM `emit-llvm` dual-path (same refuse) | **In** |
| `#[Secret]` attribute | **residual** |

## What we do **not** claim

- Full IFC / every OS or log sink  
- Attribute grammar  
- Cryptographic redaction  
- Whole secrecy story as product-green  

**Other sinks residual** (NetCap friends beyond listed, logs, non-println residual wording).

## Fail-closed residual

Do **not** claim the whole secrecy story is product-green.  
Fail-closed residual: unfinished sinks stay residual, not soft-pass.

## Rails (must stay green)

- `scripts/secret_sink_enforce_smoke.sh` — path A emit refuse  
- `scripts/secret_taint_residual_smoke.sh` — honesty  
- `scripts/secret_product_floor_smoke.sh` — umbrella  

## Phase 2 depth (M160)

**In:** `eprintln` bare SECRET IDENT refuse at check (same as println).
**Rails:** `scripts/secret_eprintln_smoke.sh`
