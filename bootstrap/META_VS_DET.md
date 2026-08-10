# Metamorphic vs deterministic builds

**Status:** residual honesty (not enforced). PM **6.1**.  
**Marker:** `META_VS_DET_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: policy tension metamorphic vs reproducible (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No metamorphic product path; input_fp is content fingerprint only (M20)

## Rails

- Doc marker: `META_VS_DET_RESIDUAL_ALPHA`
- Smoke: `scripts/meta_vs_det_residual_smoke.sh`
- Fixture: `fixtures/meta_vs_det_marker.oo` (marker comment only)

## Next (path A, not this pack)

Bounded product refuse or thin enforce path — still not full DESIGN depth.
