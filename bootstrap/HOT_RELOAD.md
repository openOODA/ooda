# Native hot-code reloading

**Status:** residual honesty (not enforced). PM **4.2**.  
**Marker:** `HOT_RELOAD_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: hot reload (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No native hot-reload product path shipped


## Path A product floor (alpha) — M153

**Path A marker:** `HOT_RELOAD_PATH_A_ALPHA`  
**Status:** path A **In** — check default-deny of named residual free calls (`check_residual.oo`).  
**In:** hot_reload/live_reload free calls refused at check  
**Rails:** `scripts/residual_path_a_floor_smoke.sh`  
**Still residual:** full DESIGN implementation of this moonshot (not claimed).

## Rails

- Doc marker: `HOT_RELOAD_RESIDUAL_ALPHA`
- Smoke: `scripts/hot_reload_residual_smoke.sh`
- Fixture: `fixtures/hotreload_marker.oo` (marker comment only)

## Next (path A, not this pack)

Bounded product refuse or thin enforce path — still not full DESIGN depth.
