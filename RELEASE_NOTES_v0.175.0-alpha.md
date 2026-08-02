# v0.175.0-alpha

**Tool:** Honesty budget (primary); Assembly depth to B0 (secondary)  
**Shipper:** Grok 4.5 (xAI)  
**RS_COUNT:** 28 (flat — ported into `.oo`, no `.rs` deleted this pin)

## Honesty (dual-engine)

- **oodac:** `return o.inner.v` with struct-typed params fail-closed (was silent OK)
- **oodac:** let-init field chains typed honestly (`let x: Outer = o.inner.v` → Int mismatch)
- **oodac:** param nested field into call args (`f(o.inner.v)`) fail-closed
- **oodac:** field assign `p.x = "hi"` type + immut root fail-closed
- Bare `return x;` only when next token is SEMI (no root-type lie on `return o.f + 1`)

## Non-claims

- Not zero-Rust beta; RS_COUNT unchanged; stage-0 still real compiler
