# Cryptographic call-graph integrity

**Status:** residual honesty (not enforced). PM **3.9**.  
**Marker:** `CALLGRAPH_CRYPTO_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: signed call-graph / integrity seals (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No call-graph crypto, signed edges, or binary integrity seals shipped

## Rails

- Doc marker: `CALLGRAPH_CRYPTO_RESIDUAL_ALPHA`
- Smoke: `scripts/callgraph_crypto_residual_smoke.sh`
- Fixture: `fixtures/callgraph_marker.oo` (marker comment only)

## Next (path A, not this pack)

Bounded product refuse or thin enforce path — still not full DESIGN depth.
