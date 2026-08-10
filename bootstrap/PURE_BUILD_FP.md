# Pure multi input fingerprint (M20)

## What it is

`scripts/oodac_pure_build.sh` records a **stable content fingerprint** of pure multi
**emit inputs** after the module list is known:

- Modules in deps-first order (`MODS[]` from import DFS; main last)
- For each module: `relpath\0` + raw file bytes
- `relpath` = path relative to repo root when under `$ROOT`, else `basename`
- Digest = **SHA-256** of the concatenated stream (hex)

Banner (stdout):

```text
pure_build: input_fp=<64 hex>
```

Optional: set `PURE_BUILD_FP_OUT=/path` to write the hex fingerprint only (one line).
The banner is also repeated just before `OK_PURE_MULTI`.

## What it is not

- **Not** a claim of bit-identical product binaries
- **Not** a full reproducible-dist / hermetic toolchain proof
- Binaries may still differ across runs or hosts due to:
  - object timestamps / build-id
  - ASLR / non-deterministic link artifacts
  - host `gcc` / libc version
  - path embedding outside the input stream

Same tree → same `input_fp` on two pure multi runs. That is the contract.

## Smoke

`scripts/pure_build_fp_smoke.sh` (wired in `scripts/ci_product.sh` after residual/seed rails):

1. Two pure multi builds of `bootstrap/corpus/import/pass/multi_ok.oo` (tiny multi-module)
2. Capture `input_fp` via `PURE_BUILD_FP_OUT` (log banner fallback)
3. Assert equal and non-empty (64 hex)
4. Copy fixture tree, touch a module source, assert fingerprint changes

## Related

| Path | Role |
|------|------|
| `scripts/oodac_pure_build.sh` | computes + prints `input_fp` |
| `scripts/pure_build_fp_smoke.sh` | stability rail |
| `scripts/seed_pure_multi_smoke.sh` | cold seed pure multi (orthogonal) |
