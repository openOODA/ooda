# Universal GPU/NPU targets

**Status:** residual honesty (not enforced). PM **4.1.3**.  
**Marker:** `GPU_NPU_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: PTX/ROCm/SPIR-V/Metal backends (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No GPU/NPU backend product path shipped
- No CUDA / OpenCL / Vulkan device dispatch product


## Path A product floor (alpha) — M153 / M165

**Path A marker:** `GPU_NPU_PATH_A_ALPHA`  
**Status:** path A **In** — check default-deny of named residual free calls (`check_residual.oo`).  
**In:** `emit_ptx` / `emit_spirv` free calls refused at check (stay residual free-name refuse)  
**Sealed product (M165):** `gpu_launch(gpu, shader)` under `&GpuCap` → `oo_gpu_launch`:

| shader | Result |
|--------|--------|
| empty or `"noop"` | **Ok**(`"gpu-noop"`) — honesty no-op, no device |
| starts with `"cpu:"` | **Ok** CPU interpret (`"cpu:add:a:b"` → `"cpu fallthrough:N"`) |
| else (PTX/SPIR-V/…) | **Err**(`"gpu residual: no device shaders"`) fail-closed |

**Dual path:** granted `gpu` → table above; zero/magic forge → `ERR\tcap\t…` + exit  
**Rails:** `scripts/gpu_path_a_smoke.sh`, `scripts/residual_path_a_floor_smoke.sh`, `scripts/libfloor_thread_gpu_smoke.sh`  
**Still residual:** full DESIGN GPU/NPU backends / real device shaders (not claimed).

## Rails

- Doc marker: `GPU_NPU_RESIDUAL_ALPHA` + `GPU_NPU_PATH_A_ALPHA`
- Smoke: `scripts/gpu_path_a_smoke.sh`, `scripts/gpu_npu_residual_smoke.sh`, `scripts/libfloor_mutex_thread_smoke.sh`
- Fixture: `fixtures/gpu_path_a.oo`, `fixtures/gpu_marker.oo`, `fixtures/libfloor_gpu_launch.oo`, `fixtures/libfloor_gpu_cap.oo`
- Std: `std/os/gpu.oo` (`gpu_launch_shader`)
- Runtime: `runtime/chs_rt_libfloor.c` (`oo_gpu_launch`)

## Next (path A, not this pack)

Real shader lower / device dispatch under GpuCap — still not full DESIGN depth.
