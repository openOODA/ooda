# Capability sandboxing vs C/C++ FFI — path A product floor (alpha)

**Marker:** `CAP_FFI_PATH_A_ALPHA`  
**Residual marker (full seal):** `CAP_FFI_RESIDUAL_ALPHA`  
**Status:** PM **6.3 done (alpha)** — check seal + process-local FFI token + allowlisted OS dlopen Path A.  
**Not claimed:** full C TCB / unrestricted any-path `dlopen` / raw-pointer / compile-time FFI gen / product `dlsym`.

## Product surface (path A In)

| Form | Behavior |
|------|----------|
| `&UnsafeFFICap` | Cap type in param lists; sealed FFI free names require it at check |
| Sealed free names | `dlopen` / `dlsym` / `dlclose` / `chs_build` / `host_*` / `ooda_host_*` |
| **Check** | Default-deny bare FFI free calls without matching `&UnsafeFFICap` + first-arg token |
| **Emit (Backend-C)** | `dlopen` → `oo_dlopen`; `dlsym` → `oo_dlsym`; `dlclose` → `oo_dlclose`; other host-FFI free names still emit residual |
| **Runtime** | Process-local `oo_cap_grant_ffi` / `oo_cap_require_ffi`; allowlisted OS `dlopen` Path A |
| `// FFI: residual` | Comment form for residual honesty (full seal still residual) |

## What is true today

| Layer | Behavior |
|-------|----------|
| **Process-local Fs/Sys/Net/…** | Seal their own ops; they do **NOT** seal arbitrary C TCB or raw pointers |
| **FFI free names (check)** | Bare `dlopen` / host-FFI free calls need explicit `&UnsafeFFICap` |
| **FFI runtime (M156/M162/M165)** | Process-local FFI token + forge deny; OS `dlopen` only under env allow rules |
| **Honesty** | This file + `cap_ffi_*` / `ffi_dlopen_path_a_smoke` |

**DESIGN tension residual:** Capability sandboxing and C/C++ FFI pull opposite directions. Path A seals named free calls and a process-local token; it does **not** sandbox the whole C runtime TCB.

**Fail-closed residual:** process-local tokens do **not** seal unrestricted OS `dlopen`, raw pointers, or every host C interop surface.

## What we do **not** claim

- “FFI fully sealed” / “FFI fully enforced” / “FFI sandbox shipped” over all C  
- Process-local Fs/Sys/… as a seal over C FFI, OS `dlopen`, or raw pointers  
- Unrestricted any-path OS `dlopen` / product `dlsym` loading under control  
- Compile-time FFI generation (`import "C" "…"`) with cap taint  
- Full DESIGN capability taint-tracking across every FFI boundary  

## Path A runtime — In

| Piece | Behavior |
|-------|----------|
| `oo_cap_grant_ffi` | Process-local token (main inject `ffi = oo_cap_grant_ffi()`) |
| `oo_cap_require_ffi` | Zero / classic magic forge → `ERR\tcap\t…` + exit |
| `oo_dlopen` | After require (M165): see allow table below |
| `oo_dlsym` / `oo_dlclose` | After require: **Err residual** stubs (`dlsym`/`dlclose` not product) |

### OS `dlopen` allow rules (M165)

Requires `OODA_FFI_ALLOW_DLOPEN=1`. Without it → residual Err after seal.

| `OODA_FFI_ALLOWDIR` | Allowed absolute paths |
|---------------------|------------------------|
| set (non-empty) | Prefix allowlist under that absolute dir (M162) |
| empty / unset | Only under `/lib`, `/lib64`, `/usr/lib`, `/usr/lib64` (safe system dirs) |

Still residual: unrestricted any-path load, raw-pointer grammar, compile-time FFI gen (`FFI_GEN.md`), product `dlsym` resolve.

**Rails:** `scripts/cap_ffi_runtime_smoke.sh`, `scripts/ffi_dlopen_path_a_smoke.sh`, `scripts/m162_residual_deepen_smoke.sh`.

## Rails

- Doc markers: `CAP_FFI_PATH_A_ALPHA` + `CAP_FFI_RESIDUAL_ALPHA`  
- Product floor: `scripts/cap_ffi_product_floor_smoke.sh`  
- Runtime: `scripts/cap_ffi_runtime_smoke.sh`  
- Path A broaden: `scripts/ffi_dlopen_path_a_smoke.sh`  
- Residual honesty: `scripts/cap_ffi_residual_smoke.sh`  
- Fixtures: `ffi_dlopen_{fail,pass,runtime_pass}.oo`, corpus `no_cap_dlopen` / `ok_unsafe_ffi_*`  
- Runtime: `runtime/chs_rt_ffi.c`
