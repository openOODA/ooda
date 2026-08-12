# Static taint (`// SECRET:`) — path A product floor (alpha) + residual

**Marker:** `SECRET_TAINT_RESIDUAL_ALPHA`  
**Status:** **Path A product floor In (alpha).** PM **3.5** → **done (alpha)** for path A.  
**Not full DESIGN taint.** Not a complete secrecy / IFC product.  
**Residual listed:** `#[Secret]` attr, full IFC, interproc beyond path A, every OS/log sink, crypto redaction, NetCap friends beyond the table, T2 scan-depth items — not soft-pass.

## Product surface (In — path A floor alpha)

| Form | Status |
|------|--------|
| `// SECRET: name` tag | **In** |
| path A bare IDENT **println/eprintln** refuse | **In** |
| Assign / concat / call prop (same file) | **In** |
| Multi-hop assign-prop chain | **In** |
| `write_file` content + path | **In** |
| `fetch` URL | **In** |
| `sys_exec` argv | **In** |
| `env_get` key | **In** |
| `read_file` / `path_exists` / `file_size` path | **In** |
| `seed` / `sleep_ms` / `alloc_bytes` / `free_bytes` / `malloc` / `free` / `realloc` values | **In** |
| `process_exit` code | **In** |
| setenv / env_set / unsetenv / mmap-family OS sinks (named) | **In** (partial; re-prove rails) |
| LLVM `emit-llvm` dual-path (same refuse) | **In** |
| Function return taint via `__fr_secret__` | **In** (path A) |
| Alias chain for sinks (`__alias__` walk) | **In** (path A) |
| Sticky clear on clean rebind | **In** (path A) |
| `#[Secret]` attribute | **residual** |

## Path A depth (named mechanisms)

These are the path-A claims for the current emit/check secret floor. They are **not** full DESIGN taint analysis.

### 1. Function return taint via `__fr_secret__`

- Pre-pass walks each fn body for `// SECRET:` directive lines and bare IDENTs already tagged `__sec__name=1`.
- When a body is secret-bearing, env records `__fr_secret__<fn_name>=1`.
- Assign/let prop (`c_secret_prop_from_rhs`) treats `callee(...)` as secret when `__fr_secret__callee` is set, so `let y = make_secret(); println(y)` refuses.
- **Path A only:** same-file free-fn return flag + IDENT call sites. Not a full interprocedural lattice.

### 2. Alias chain for sinks

- Bare rebind `name = other;` (IDENT RHS only) records `__alias__name=other` at emit.
- Sink refuse resolves through the chain (`c_secret_refuse_sink_resolved`, depth-capped) before `__sec__` check.
- So aliases that still name a SECRET root are refused at sealed sinks (println, write_file, fetch, sys_exec, …).
- **Path A only:** simple IDENT→IDENT alias map for sink resolve — not full points-to / heap aliasing.

### 3. Sticky clear on clean rebind

- Taint is **sticky** through secret-bearing RHS (assign/let prop keeps `__sec__dest=1`).
- A **clean rebind** (RHS with no secret IDENT / no `__fr_secret__` callee) **clears** the dest tag so later sinks may accept the name.
- Example path A: `let mut y = api_key; y = "clean"; println(y)` — allowed after clear.
- Example path A: `y = api_key; println(y)` — still refuse (rebind *to* secret).
- **Path A only:** local name tag clear on clean RHS — not cryptographic wipe, not object-field deep clear.

## What we do **not** claim

- Full DESIGN / full IFC taint  
- Taint tracking **shipped/enforced** as a complete product story  
- Full interprocedural analysis beyond path-A `__fr_secret__`  
- **T2 scan depth** as done (do **not** claim): full `args` string scan completeness, nested RHS walker exhaustiveness, string/comment false-positive hardening, **field / index / method-return** taint, FFI payload-arg taint, closure-return lattice, string-interp `${}` deep taint  
- Attribute grammar `#[Secret]`  
- Cryptographic redaction  
- Whole secrecy story as product-green  

**Other sinks residual** (NetCap friends beyond listed, logs, non-println residual wording).

## Residual (beyond path A)

| Area | Status |
|------|--------|
| Full IFC / secrecy lattice | **residual** |
| Interproc beyond path-A `__fr_secret__` (cross-module, parametric, higher-order) | **residual** |
| Field / index / method-return / closure-return taint (T2) | **residual** — not claimed done |
| Nested scan / false-positive hardening (T2) | **residual** — not claimed done |
| FFI / dlopen payload taint (T2) | **residual** — not claimed done |
| `#[Secret]` attr | **residual** |
| Crypto redaction / wipe of secret bytes | **residual** (runtime crypto wipe is separate) |
| NetCap / non-println friends beyond the In table | **residual** |

## Fail-closed residual

Do **not** claim the whole secrecy story is product-green.  
Fail-closed residual: unfinished sinks and T2 depth stay residual, not soft-pass.  
Path A is a named emit/check floor — **not** full DESIGN taint.

## Rails (must stay green)

- `scripts/secret_sink_enforce_smoke.sh` — path A emit refuse  
- `scripts/secret_taint_residual_smoke.sh` — honesty  
- `scripts/secret_product_floor_smoke.sh` — umbrella  
- `scripts/secret_eprintln_smoke.sh` — eprintln dual  

## Phase 2 depth (M160)

**In:** `eprintln` bare SECRET IDENT refuse at check (same as println).  
**Rails:** `scripts/secret_eprintln_smoke.sh`
