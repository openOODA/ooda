# METAMORPHIC — immune / layout diversity residual pack

**Marker:** `METAMORPHIC_RESIDUAL_ALPHA`

## Path A (alpha product floor — limited)

1. **Emit-time layout decoys (opt-in)**  
   Source directive (line-start only):
   ```
   // METAMORPHIC: path-a
   ```
   When present, Backend-C emit inserts non-semantic `__oo_meta_decoy_N` functions after each top-level fn so the **exposed C/object layout** differs from a build without the marker.  
   - Default **OFF** (no directive) → stable layout for deterministic product builds.  
   - Does **not** re-mutate code after load.  
   - Fail-closed residual wording: this is **not** full DESIGN runtime assembly mutation.

2. **Process-local epoch**  
   Free name `meta_epoch()` → `oo_meta_epoch()`: fixed random 64-bit value for the process (first call). For future immune hooks / diversification seeds.  
   Residual: does not rewrite .text at runtime.

## Residual (not product — free-name refuse)

| Free name | Tag |
|-----------|-----|
| `metamorphic_emit` | METAMORPHIC |
| `metamorphic_build` | METAMORPHIC |
| `metamorphic_vs_det` | META_VS_DET |

Full DESIGN “polymorphic metamorphic binaries / immune systems” (continuous RAM re-mutation, ROP graph reshape) is **not shipped**.  
Fail-closed residual at check: residual free-name default-deny.

## Tension with deterministic builds

- Source `input_fp` remains the reproducibility anchor.  
- Path-A decoys are **explicit opt-in** so default CI/product builds stay layout-stable.  
- True bit-identical binary + continuous polymorphism remains residual (DESIGN §6.1).

## Honesty

Honesty ban list (never product-green without residual tag): runtime code re-mutation is residual only.  
Claim only: path-A layout decoys + `meta_epoch`; full immune system remains residual.
