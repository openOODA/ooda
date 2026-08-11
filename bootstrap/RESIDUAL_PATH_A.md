# Residual free-name path A product floor (alpha) — M153

**Marker:** `RESIDUAL_PATH_A_ALPHA`  
**Status:** path A **In** for remaining DESIGN moonshot free-call surfaces.

## What is production-ready (alpha)

Unshipped DESIGN free calls are **default-deny at check**:

| Surface | Example free names refused |
|---------|----------------------------|
| HITL | `verify_human` |
| TEMPORAL | `checkpoint`, `rollback`, `snapshot_state` |
| HIVEMIND | `hive_fuzz`, `hivemind_join` |
| HOT_RELOAD | `hot_reload`, `live_reload` |
| SHADOW_STATE | `shadow_revert`, `shadow_commit` |
| METAMORPHIC | `metamorphic_emit`, `metamorphic_build` |
| HOLOGRAPHIC | `holo_persist`, `holo_load` |
| GPU_NPU | `emit_ptx`, `emit_spirv` (sealed residual: `gpu_launch` + `&GpuCap`) |
| BARE_METAL | `bare_metal_init` |
| AST_MACROS | `macro_expand`, `ast_macro` |
| TYPE_STATE | `type_state_check`, `typestate_assert` |
| DOD_LAYOUT | `soa_layout`, `dod_layout` |
| TELEPATHIC_AST | `telepathic_compile`, `intent_compile` |
| NATIVE_LSP | `lsp_serve` |
| CONCURRENCY | `actor_spawn` (channels sealed under ThreadCap M164) |
| CALLGRAPH_CRYPTO | `sign_callgraph`, `verify_callgraph` |
| FFI_GEN | `ffi_gen`, `import_c` |
| LTO_XLANG | `lto_xlang_link` |
| TOOLCHAINS_ADV | `advanced_toolchain` |
| PLAYGROUND | `playground_eval` |
| META_VS_DET | `metamorphic_vs_det` |

Codes: `ERR\tresidual\t…` → `--json-errors` **E_RESIDUAL**.

## What we do **not** claim

Full DESIGN implementations of any moonshot above. Path A is **fail-closed refuse**, not feature complete.

## Rails

- `oodac/check_residual.oo`
- `scripts/residual_path_a_floor_smoke.sh`
- Per-pack `*_PATH_A_ALPHA` markers

