# Static taint (`// SECRET:`) — product surface + residual

**Marker:** `SECRET_TAINT_RESIDUAL_ALPHA`  
**Status:** multiple sinks **In**. PM **3.5** stays **partial**.  
**Sprint:** M128–M131, M135, **M140–M143** (env_get, read_file, path_exists, file_size).

## Product surface (In)

| Form | Status |
|------|--------|
| `// SECRET: name` tag | **In** |
| `println` bare secret IDENT | **In** |
| Assign / concat / call prop | **In** |
| `write_file` content IDENT | **In** |
| `fetch` URL IDENT | **In** |
| `sys_exec` argv IDENT | **In** |
| `env_get` key IDENT | **In** (M140) |
| `read_file` path IDENT | **In** (M141) |
| `path_exists` path IDENT | **In** (M142) |
| `file_size` path IDENT | **In** (M143) |
| LLVM dual-path check | **In** (M129) |
| `#[Secret]` attribute | **residual** |

## What we do **not** claim

- Full IFC / all OS or log sinks  
- Attribute grammar  
- Cryptographic redaction  

**Other sinks residual** (NetCap friends beyond listed, logs, etc.).

## Rails

- Marker: `SECRET_TAINT_RESIDUAL_ALPHA`
- Enforce: `scripts/secret_sink_enforce_smoke.sh`
- Residual: `scripts/secret_taint_residual_smoke.sh`
