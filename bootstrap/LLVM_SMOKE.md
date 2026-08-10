# LLVM backend residual
**Status:** residual honesty (smoke depth only). PM **4.1.2**. **Marker:** `LLVM_SMOKE_RESIDUAL_ALPHA`
## Named / partial surface
Execute smokes exist when toolenv present; not production floor.
## Fail-closed residual
Do not treat smoke backends as production floors.
## What we do **not** claim
Not a production LLVM/WASM floor; toolenv-gated execute only.
## Rails
- `LLVM_SMOKE_RESIDUAL_ALPHA`
- `scripts/llvm_smoke_residual_smoke.sh`
