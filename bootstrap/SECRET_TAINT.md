# Static taint (`// SECRET:`) — product surface + residual

**Marker:** `SECRET_TAINT_RESIDUAL_ALPHA`  
**Status:** many sinks **In**. PM **3.5** stays **partial**.  
**Sprint:** M128–M151 family (println through process_exit extras).

## Product surface (In)

| Form | Status |
|------|--------|
| `// SECRET: name` tag | **In** |
| path A bare IDENT **println** refuse | **In** |
| Assign / concat / call prop | **In** |
| `write_file` content + **path** | **In** |
| `fetch` URL | **In** |
| `sys_exec` argv | **In** |
| `env_get` key | **In** |
| `read_file` / `path_exists` / `file_size` path | **In** |
| `seed` / `sleep_ms` / `alloc_bytes` / `free_bytes` values | **In** |
| `process_exit` code | **In** |
| LLVM dual-path check | **In** |
| `#[Secret]` attribute | **residual** |

## What we do **not** claim

- Full IFC / every OS or log sink  
- Attribute grammar  
- Cryptographic redaction  

**Other sinks residual** (NetCap friends beyond listed, logs, non-println residual wording).

## Fail-closed residual

Do **not** claim the whole secrecy story is product-green.  
Fail-closed residual: unfinished sinks stay residual, not soft-pass.

## Rails

- Marker: `SECRET_TAINT_RESIDUAL_ALPHA`
- Enforce: `scripts/secret_sink_enforce_smoke.sh`
- Residual: `scripts/secret_taint_residual_smoke.sh`
