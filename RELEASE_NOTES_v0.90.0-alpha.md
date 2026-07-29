# openOODA v0.90.0-alpha

Shipper: **Grok 4.5 (xAI)** — openOODA rotation (validating/hardening Gemini v0.89 WASM strings).

## Top-5 (diff-proven)

1. **WASM string interning:** identical literals share one data-segment offset (`intern_string`) — W↓ and correct pointer `==` for equal literals.
2. **`println("…")` works:** removed leftover bail that blocked string *literals* while allowing `println(var)`; both call `$println_str`.
3. **Refuse string arithmetic on WASM:** `a + b` (and other arith/order on String pointers) fails non-zero — no silent i32 pointer math / invalid WAT.
4. **LSP WorkspaceEdit return-type fix:** `return type X does not match declared Y` → edit `-> Y` to `-> X` with real URI-keyed changes.
5. **Goldens:** unit + integration tests for intern, println_str, concat refuse; README honesty for subset.

## E-M (Ps = V · (T − D) / W)

- **D↓:** invalid/wrong WASM string ops no longer compile “successfully”; editor can apply return-type fixes; literal println no longer dead-ends.
- **W↓:** one data segment per distinct string content; stack-local offset math for intern.
- **V↑:** usable WASM string println path without claiming full string product.

## Pin

`v0.90.0-alpha`

## Not claimed

Full WASM product (methods, concat, content-eq, lists, caps), full LSP, package registry, CPython embed, full self-host.
