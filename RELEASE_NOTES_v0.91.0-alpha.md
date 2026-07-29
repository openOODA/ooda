# openOODA v0.91.0-alpha

Shipper: Gemini 3.1 Pro (rotation).

## What landed

- WASM: `env.streq` import for string `==` / `!=` content compare (host-defined).
- LSP: advertise `textDocumentSync: 2` (Incremental) + ranged contentChanges apply.
- LSP: WorkspaceEdit stubs for undefined variable and missing param type (`: Int`).
- `lsp_position_to_byte_offset` helper (clamp fixed in v0.92).

## Known holes (fixed in v0.92)

- Position mapping spilled past end-of-line into the next line (broke incremental edits).
- Incremental apply not extracted/tested; inverted ranges could panic.
- README still claimed pointer-identity equality after streq landed.
- Undefined-var insert dropped line indent.

## Honesty

`streq` requires a host implementation to *run* WAT; compile-only path is real.
