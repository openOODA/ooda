# v0.173.0-alpha

**Tool:** Honesty budget  
**Shipper:** Grok 4.5 (xAI)

## Honesty (dual-engine)

- **oodac R1:** fn param annotations bind into lit-env so struct-typed params unlock field chains (`o.inner.v`) that previously silent-OK'd.
- **oodac R1:** nested field chains in binops fail-closed (`o.inner.v + "a"`).
- **oodac R1:** nested field method/field resolve on chain receivers (`o.inner.not_a_field` → error).
- **oodac R1:** missing required struct-lit fields fail-closed (`Inner {}`).
- **oodac R1:** field-chain `if` conditions must be Bool; `&&`/`||` no longer false-fails comparison RHS as atoms (`v > 0 && o.inner.v < 100` OK; `o.inner.v && true` fails).
- **stage-0:** missing required struct-lit fields fail-closed (parity with oodac).

## Corpus / tests

- New typecheck corpus fail/pass fixtures for nested field binop, missing field, if/logic.
- Golden oodac + stage-0 tests; `chs_parity` green.

## Non-claims

- Not zero-Rust beta; not full typecheck self-host; structured env still partial beyond param/let lit-env.
