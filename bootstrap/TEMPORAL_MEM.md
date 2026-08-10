# Temporal memory (state rollback)

**Status:** residual honesty (not enforced). PM **3.8**.  
**Marker:** `TEMPORAL_MEM_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: temporal snapshot / rollback APIs (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No time-travel state, snapshot/restore runtime, or temporal cap shipped

## Rails

- Doc marker: `TEMPORAL_MEM_RESIDUAL_ALPHA`
- Smoke: `scripts/temporal_mem_residual_smoke.sh`
- Fixture: `fixtures/temporal_marker.oo` (marker comment only)

## Next (path A, not this pack)

Bounded product refuse or thin enforce path — still not full DESIGN depth.
