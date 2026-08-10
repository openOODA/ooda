# M25 Capability sandboxing vs C/C++ FFI — residual at alpha

**Marker:** `CAP_FFI_RESIDUAL_ALPHA`  
**Status:** residual honesty (not enforced). PM **6.3** / sprint **M25**.

## Product surface (names only)

| Form | Intent |
|------|--------|
| `&UnsafeFFICap` | DESIGN named surface: deliberate sandbox breach for C/C++ FFI |
| `// FFI: residual` | Simpler product marker (comment form) |

Either form **names** the FFI breach surface. At alpha neither is a type, grant, or emit gate — **doc name only** for `&UnsafeFFICap` (not required to implement the type).

## What is true today

| Layer | Behavior |
|-------|----------|
| **Process-local caps** | FS/Sys/Env/Net/Time/Rand/Alloc sealed ops re-check process-local tokens (see `STATIC_CAPS.md`) |
| **C FFI / dlopen / raw pointers** | Process-local caps do **NOT** seal C FFI, `dlopen`, or raw-pointer escape |
| **Parse / check** | No `&UnsafeFFICap` type or FFI-breach grammar; comment marker is source-level only |
| **Runtime floor** | Backend-C always links `chs_rt` (C TCB, not user FFI). Optional `OODA_WITH_HOST_FFI` is uncapped host C interop — **not** gated by `&UnsafeFFICap` |
| **Honesty** | Residual documented here + `scripts/cap_ffi_residual_smoke.sh` |

**DESIGN tension residual:** Capability sandboxing and C/C++ FFI pull opposite directions. DESIGN requires explicit `&UnsafeFFICap` for Compile-Time FFI; product alpha has **not** shipped that seal.

**Fail-closed residual:** do not treat process-local caps or presence of `// FFI: residual` / `&UnsafeFFICap` as an FFI boundary. Untrusted code that reaches C / `dlopen` / raw pointers is **not** sealed by the product caps ladder.

## What we do **not** claim

- “FFI fully sealed” / “FFI fully enforced” / “FFI sandbox shipped”  
- Process-local caps as a seal over C FFI, `dlopen`, or raw pointers  
- Implemented `&UnsafeFFICap` type / grant / runtime token  
- Compile-time FFI generation (`import "C" "…"`) with cap taint  
- Full DESIGN capability taint-tracking across the FFI boundary

## Rails

- Doc marker: this file must contain `CAP_FFI_RESIDUAL_ALPHA`
- Smoke: `scripts/cap_ffi_residual_smoke.sh` (wired in `ci_product`)
- Fixture (marker only, not enforced): `fixtures/ffi_marker.oo`

## Next (not this sprint)

Path A candidate: name-check refuse of raw `dlopen` / host-FFI symbols without a documented `&UnsafeFFICap` param form — still **not** full FFI sandbox or process-local seal over C.
