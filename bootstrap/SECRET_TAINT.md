# M22/M52 Static taint (`#[Secret]`) — path A In + residual

**Marker:** `SECRET_TAINT_RESIDUAL_ALPHA`  
**Status:** **M52 path A In** (Backend-C `println` bare-IDENT refuse); residual honesty for the rest. PM **3.5** / sprint **M22/M52**.

## Product surface

| Form | Intent | Status |
|------|--------|--------|
| `// SECRET: name` | Line-start comment tags name as secret | **In** (path A) |
| `#[Secret]` | DESIGN attribute on sensitive vars | **residual** (name only; no grammar) |

## What is true today

| Layer | Behavior |
|-------|----------|
| **Parse / check** | No `#[Secret]` grammar; **M55/M60:** `oodac check` dual-path path A names + path B bare-IDENT assign-prop sim at `println` |
| **Path A (In)** | Backend-C `emit-c`: file-level line-start `// SECRET: name` tags; bare `println(ident)` refused with `ERR\tsecret` when ident is tagged |
| **Path B (In)** | Direct bare-IDENT assign-prop: `let y = x` / `y = x` copies secret tag when `x` is tagged; multi-hop chains (`z = y` after `y = x`) |
| **Taint analysis** | Interprocedural taint (call arguments, function returns) + concat/expr propagation |
| **Other sinks** | **No** NetCap / log / file Secret gate; only `println` arguments checked |
| **Honesty** | Residual documented here + residual + enforce smokes |

**Fail-closed residual:** do **not** treat `// SECRET: name` or `#[Secret]` as a full confidentiality boundary. Path A is a narrow emit refuse — **not** full static taint analysis.

## What we do **not** claim

- Interprocedural full-program alias analysis
- Secret→public sink refuse for NetCap / log / non-println sinks as product green  
- Full IFC / “taint tracking shipped” / “taint tracking enforced”  
- Full DESIGN AST-flow mathematical guarantee for passwords  
- Cryptographic redaction or runtime secret scrubbing  
- `#[Secret]` attribute enforcement  

## Rails

- Doc marker: this file must contain `SECRET_TAINT_RESIDUAL_ALPHA`
- Residual smoke: `scripts/secret_taint_residual_smoke.sh` (wired in `ci_product`)
- Enforce smoke: `scripts/secret_sink_enforce_smoke.sh` (M52 path A)
- Fixtures: `secret_sink_fail.oo`, `secret_sink_pass.oo`, `secret_marker.oo`, `secret_assign_*`, `secret_chain_fail.oo`, `secret_concat_fail.oo`, `secret_invalid_empty.oo`

**Assign-prop note:** bare-IDENT copy tags destination; reassign to a public literal does **not** clear the tag (sticky fail-closed). Clearing/untaint residual.

## Next (not this sprint)

NetCap sinks, check dual-path, `#[Secret]` attribute grammar — still residual.
