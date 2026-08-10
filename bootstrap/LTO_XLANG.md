# Cross-language LTO

**Status:** residual honesty (not enforced). PM **4.3.1**.  
**Marker:** `LTO_XLANG_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: C++/Rust LTO (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No cross-language LTO product path shipped

## Rails

- Doc marker: `LTO_XLANG_RESIDUAL_ALPHA`
- Smoke: `scripts/lto_xlang_residual_smoke.sh`
- Fixture: `fixtures/lto_marker.oo` (marker comment only)

## Next (path A, not this pack)

Bounded product refuse or thin enforce path — still not full DESIGN depth.
