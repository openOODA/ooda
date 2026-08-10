# LLVM backend — production emitter surface (M119)
**Status:** Production emit→link→run **In** for proven surface. PM **4.1.2**. **Marker:** `LLVM_SMOKE_RESIDUAL_ALPHA` (residual = beyond proven surface / self-host)
## Named / In surface
- **CHS fixtures:** `chs_hello`, `while_count`, `for_range`, `chs_list_string` — O0/O3 parity vs Backend-C (`llvm_prod_parity_smoke`).
- **Multi-module/import:** `fixtures/m119_multi` (import expand + user fn ret types) — O0/O3 parity.
- **Product path:** `ooda build --target llvm` → `llvm_link` **-O0**; **`--release` → -O3**.
- **Link recipe:** `scripts/llvm_link.sh` (fail-closed `ERR_NO_LLVM`).
- **M129 Secret:** `emit-llvm` runs the same Secret dual-path check as Backend-C before IR (println + write_file sinks refuse).
- **Immune:** unsupported non-range `for` residual ERR; missing tools non-zero; secret sink non-zero.
- List System V sret/byval; if-expr; range-for; unary `!`; user-fn return table.
## Fail-closed residual
Do not treat LLVM as full oodac **self-host** floor (Backend-C remains product self-host).
Full C-emit parity (match/Secret/MaxCycles/contracts multi-module self-host) residual beyond proven surface.
Missing clang/llc → `ERR_NO_LLVM` (never soft-pass).
## What we do **not** claim
Not alternate self-host; not GPU/NPU; not cross-lang LTO; not full language surface.
## Rails
- `LLVM_SMOKE_RESIDUAL_ALPHA`
- `scripts/llvm_smoke_residual_smoke.sh`
- `scripts/llvm_link.sh`
- `scripts/llvm_prod_parity_smoke.sh`
- `scripts/llvm_fail_closed_smoke.sh`
- `scripts/llvm_execute_smoke.sh`
