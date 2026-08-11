# M152 Capability sandboxing vs C/C++ FFI — path A product floor (alpha)

**Marker:** `CAP_FFI_PATH_A_ALPHA`  
**Residual marker (full seal):** `CAP_FFI_RESIDUAL_ALPHA`  
**Status:** PM **6.3 done (alpha)** path A — check seal under `&UnsafeFFICap`.  
**Not claimed:** full C TCB / `dlopen` runtime / raw-pointer / compile-time FFI gen.

## Product surface (path A In)

| Form | Behavior |
|------|----------|
| `&UnsafeFFICap` | Cap type accepted in param lists; sealed FFI free names require it |
| Sealed free names | `dlopen` / `dlsym` / `dlclose` / `chs_build` / `host_*` / `ooda_host_*` |
| **Check** | Default-deny: call needs matching `&UnsafeFFICap` param **and** token as first arg (or method receiver) — same form as other sealed caps |
| **Emit (Backend-C)** | Fail-closed residual lower: no C `dlopen` / host-FFI emit (`ERR\tc_emit\tffi residual`) |
| `// FFI: residual` | Comment form for residual honesty (full seal still residual) |

## What is true today

| Layer | Behavior |
|-------|----------|
| **Process-local caps (Fs/Sys/…)** | Still seal their own ops; they do **NOT** seal arbitrary C TCB, raw pointers, or full FFI |
| **FFI escape names (path A)** | **Do** require explicit `&UnsafeFFICap` at **check** — no ambient bare `dlopen` / host-FFI free call |
| **Runtime** | No process-local `UnsafeFFICap` token / no real `dlopen` lower — emit residual fail-closed |
| **Honesty** | This file + `scripts/cap_ffi_product_floor_smoke.sh` + residual pack rail |

**DESIGN tension residual:** Capability sandboxing and C/C++ FFI pull opposite directions. Path A names the breach and refuses ambient FFI free calls. Full compile-time FFI with cap taint across the boundary is still residual.

**Fail-closed residual:** process-local Fs/Sys/… caps are **not** a seal over the whole C runtime TCB, `dlopen` at OS level, or raw-pointer escape. Path A only seals the **named free-call surface** at check.

## What we do **not** claim

- “FFI fully sealed” / “FFI fully enforced” / “FFI sandbox shipped” over all C  
- Process-local Fs/Sys/… as a seal over C FFI, OS `dlopen`, or raw pointers  
- Runtime process-local `UnsafeFFICap` token / forge deny for real `dlopen`  
- Compile-time FFI generation (`import "C" "…"`) with cap taint  
- Full DESIGN capability taint-tracking across every FFI boundary  

## Rails

- Doc markers: `CAP_FFI_PATH_A_ALPHA` + `CAP_FFI_RESIDUAL_ALPHA`  
- Product floor: `scripts/cap_ffi_product_floor_smoke.sh` (in `ci_product` / caps floor)  
- Residual honesty: `scripts/cap_ffi_residual_smoke.sh`  
- Fixtures: `fixtures/ffi_dlopen_{fail,pass}.oo`, corpus `no_cap_dlopen` / `ok_unsafe_ffi_*`  

## Residual next (not this floor)

Runtime `oo_cap_grant_ffi` + real or stub `dlopen` lower; raw-pointer grammar; compile-time FFI gen (`FFI_GEN.md`).
