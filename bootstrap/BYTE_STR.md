# Byte primitive + `&str` borrow — residual honesty (library path A)

**Status:** residual honesty (not DESIGN-done). Library residual from SPRINT backlog.  
**Marker:** `BYTE_STR_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Native **`&str`** (borrowed string slices) — not a product type
- **`Byte`** as a true u8 primitive / byte arrays — not product-lowered
- Free aspirational APIs (`str_borrow`, `as_bytes`, `byte_array_*`) — not product free names

## What is true today

| Layer | Behavior |
|-------|----------|
| **Types** | Product string surface is owned **`String`** (`OoStr` value + ARC). Value-copy / retain-release remains. |
| **Slice** | `str_slice` / `str_sub` / `char_at` return **new `String` values**, not borrowed `&str`. |
| **Bytes** | Runtime `OoStr` is length-prefixed bytes internally; there is **no** language `Byte` / `List[Byte]` / `&[Byte]` product type. |
| **std floor** | Optional docs-only `type Byte = Int` convention in `std/byte.oo` (0..255 clamp helpers). **Not** a sealed u8 primitive; **not** byte arrays. |
| **byte_at (M162)** | Free name `byte_at(s, i) -> Int` returns raw 0..255 or -1 OOB. **Not** `&str` borrow; **not** `List[Byte]`. |

## Fail-closed residual

Do **not** treat `Byte` / `&str` as DESIGN-complete product features. Absence of native borrow and byte arrays is residual, not silent green. Crypto / encoding modules that need zero-copy bytes stay blocked on this residual.

## What we do **not** claim

- Native `&str` borrow semantics (no lifetime / no shared slice type)
- Real `Byte` primitive distinct from `Int` (range typecheck, u8 ABI)
- Byte arrays (`List[Byte]`, fixed `[Byte; N]`, `&[Byte]`)
- Elimination of string value-copy overhead via borrow
- DESIGN-done for pure cryptography/encoding on byte buffers

## Path A residual floor (alpha)

**In (honesty rails only):**

- This pack documents the residual so agents do not fake DESIGN-done
- `std/byte.oo` may offer **Int-backed** 0..255 helpers only (docs convention)
- Check remains fail-closed for unknown free names; no half-baked LLVM/C byte types

**Still residual (not this pack):**

- Compiler + runtime + emit for `&str` and true `Byte` arrays
- Free-name product APIs for zero-copy string/byte views

## Rails

- Doc marker: `BYTE_STR_RESIDUAL_ALPHA`
- Smoke: `scripts/byte_str_residual_smoke.sh`
- Fixture: `fixtures/byte_str_marker.oo` (marker comment only)
- Optional floor: `std/byte.oo` (Int convention only; residual documented in-file)

## Next (not claimed here)

Native borrow type + Byte ABI in typecheck / Backend-C / LLVM; pure crypto on byte buffers.
