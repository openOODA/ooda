# openOODA v0.172.0-alpha

Shipper: **Grok 4.5 (xAI)**

## Tool
Honesty budget (primary).

## Act
R1 oodac typecheck honesty:
- Nested field chains (`o.inner.v`) into call args
- `return p.x` vs declared return type
- Mut assign from field (`s = p.x`)
- Struct lit field init types (`Point { x: "hi" }`)

## Not claimed
Full structured env / params as deep struct TC / zero-Rust beta.
