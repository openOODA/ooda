# openOODA v0.170.0-alpha

Shipper: **Grok 4.5 (xAI)**

## Tool
Honesty budget (primary); E-M secondary for D↓ silent-OK.

## Act
R1 oodac typecheck honesty:
- Method arity fail-closed (`.len(1)`, `.contains()`, `.char_at` arg count)
- Struct field table: known `type T = struct { … }` receiver — unknown fields fail-closed; known fields OK
- Struct field decls / struct-lit constructors no longer false-fail as undefined vars
- Multi-arg call fixtures locked (`f(a,b)` lit/call mismatch + OK)

## Not claimed
Full structured type env (field types into call args), zero-Rust beta, full WASM/LLVM product surfaces.
