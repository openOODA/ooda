# Capability sandboxing vs C/C++ FFI — path A product floor (alpha)

**Marker:** `CAP_FFI_PATH_A_ALPHA`  
**Residual marker (full seal):** `CAP_FFI_RESIDUAL_ALPHA`  
**Status:** PM **6.3 done (alpha)** — check seal + process-local FFI token + stub lower.  
**Not claimed:** full C TCB / OS `dlopen` isolation / raw-pointer / compile-time FFI gen.

## Product surface (path A In)

| Form | Behavior |
|------|----------|
| `&UnsafeFFICap` | Cap type in param lists; sealed FFI free names require it at check |
| Sealed free names | `dlopen` / `dlsym` / `dlclose` / `chs_build` / `host_*` / `ooda_host_*` |
| **Check** | Default-deny bare FFI free calls without matching `&UnsafeFFICap` + first-arg token |
| **Emit (Backend-C)** | `dlopen(ffi, path)` → `oo_dlopen`; other host-FFI free names still emit residual |
| **Runtime** | Process-local `oo_cap_grant_ffi` / `oo_cap_require_ffi`; `oo_dlopen` stub returns Err after seal (not OS `dlopen`) |
| `// FFI: residual` | Comment form for residual honesty (full seal still residual) |

## What is true today

| Layer | Behavior |
|-------|----------|
| **Process-local Fs/Sys/Net/…** | Seal their own ops; they do **NOT** seal arbitrary C TCB or raw pointers |
| **FFI free names (check)** | Bare `dlopen` / host-FFI free calls need explicit `&UnsafeFFICap` |
| **FFI runtime (M156)** | Process-local FFI token + forge deny; stub `oo_dlopen` after require — **not** OS library load |
| **Honesty** | This file + `cap_ffi_product_floor_smoke` / `cap_ffi_runtime_smoke` / residual smoke |

**DESIGN tension residual:** Capability sandboxing and C/C++ FFI pull opposite directions. Path A seals named free calls and a process-local token; it does **not** sandbox the whole C runtime TCB.

**Fail-closed residual:** process-local tokens do **not** seal OS `dlopen`, raw pointers, or every host C interop surface.

## What we do **not** claim

- “FFI fully sealed” / “FFI fully enforced” / “FFI sandbox shipped” over all C  
- Process-local Fs/Sys/… as a seal over C FFI, OS `dlopen`, or raw pointers  
- Real OS `dlopen` / `dlsym` loading of shared libraries under product control  
- Compile-time FFI generation (`import "C" "…"`) with cap taint  
- Full DESIGN capability taint-tracking across every FFI boundary  

## Path A runtime (M156) — In

| Piece | Behavior |
|-------|----------|
| `oo_cap_grant_ffi` | Process-local token (main inject `ffi = oo_cap_grant_ffi()`) |
| `oo_cap_require_ffi` | Zero / classic magic forge → `ERR\tcap\t…` + exit |
| `oo_dlopen` | After require: returns Err stub string; **no** OS `dlopen` |

**Rails:** `scripts/cap_ffi_runtime_smoke.sh` (pass + zero/magic forge deny).

## Rails

- Doc markers: `CAP_FFI_PATH_A_ALPHA` + `CAP_FFI_RESIDUAL_ALPHA`  
- Product floor: `scripts/cap_ffi_product_floor_smoke.sh`  
- Runtime: `scripts/cap_ffi_runtime_smoke.sh`  
- Residual honesty: `scripts/cap_ffi_residual_smoke.sh`  
- Fixtures: `ffi_dlopen_{fail,pass,runtime_pass}.oo`, corpus `no_cap_dlopen` / `ok_unsafe_ffi_*`  

## Residual next (not this floor)

**M162 path A:** OS `dlopen` when `OODA_FFI_ALLOW_DLOPEN=1` and path under absolute `OODA_FFI_ALLOWDIR` (returns `handle:…`); otherwise residual Err after seal.

Still residual: unrestricted OS load, raw-pointer grammar, compile-time FFI gen (`FFI_GEN.md`), `dlsym` product surface.
