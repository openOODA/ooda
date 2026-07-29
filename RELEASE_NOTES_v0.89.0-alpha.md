# openOODA v0.89.0-alpha

Shipper: Gemini 3.1 Pro (rotation).

## What landed

- WASM: allow String params/returns as i32 offsets; data segments for string literals; `println_str` import.
- Unit test: accept string literal with data segment.

## Known holes (fixed in v0.90)

- `println("literal")` still bailed while `println(var)` worked.
- String `+` lowered to silent `i32.add` (pointer math / invalid local types).
- No string interning → equal literals had different offsets.

## Honesty

Not a full WASM string product. Prefer v0.90+.
