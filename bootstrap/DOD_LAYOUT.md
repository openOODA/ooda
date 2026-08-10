# Data-oriented design layout

**Status:** residual honesty (not enforced). PM **1.3**.  
**Marker:** `DOD_LAYOUT_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: SoA / zero-copy DOD layout (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No SoA/DOD layout product path shipped

## Rails

- Doc marker: `DOD_LAYOUT_RESIDUAL_ALPHA`
- Smoke: `scripts/dod_layout_residual_smoke.sh`
- Fixture: `fixtures/dod_marker.oo` (marker comment only)

## Next (path A, not this pack)

Bounded product refuse or thin enforce path — still not full DESIGN depth.
