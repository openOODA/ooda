# Byte primitive + `&str` borrow — residual honesty (library path A)

**Status:** residual honesty (not DESIGN-done). Library residual from SPRINT backlog.  
**Marker:** `BYTE_STR_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Native **`&str`** (borrowed string slices with lifetimes) — not a product type
- **`Byte`** as a true u8 primitive / byte arrays — not product-lowered
- Free aspirational APIs (`str_borrow`, `as_bytes`, `byte_array_*`) — not product free names
- **`List[Byte]` ABI** — not product

## What is true today

| Layer | Behavior |
|-------|----------|
| **Types** | Product string surface is owned **`String`** (`OoStr` value + ARC). Value-copy / retain-release remains. |
| **Slice (UTF-8)** | `str_slice` / `str_sub` / `char_at` return **new `String` values**, not borrowed `&str`. |
| **Bytes** | Runtime `OoStr` is length-prefixed bytes internally; there is **no** language `Byte` / `List[Byte]` / `&[Byte]` product type. |
| **std floor** | `type Byte = Int` convention in `std/byte.oo` (0..255 clamp helpers) + thin wrappers over path A free names. **Not** a sealed u8 primitive; **not** byte arrays. |
| **byte_at (M162)** | Free name `byte_at(s, i) -> Int` returns raw 0..255 or -1 OOB. **Not** `&str` borrow; **not** `List[Byte]`. |
| **bytes_len / byte_slice / bytes_eq (M163 path A)** | Free names: `bytes_len(s) -> Int` (byte length); `byte_slice(s, start, end) -> String` (**owned** copy of raw bytes `[start,end)` by **byte index**); `bytes_eq(a, b) -> Bool`. Path A is honest owned copy — **not** true borrowed `&str`. |

## Fail-closed residual

Do **not** treat native `&str` lifetimes / `List[Byte]` as DESIGN-complete product features. Path A owned byte-indexed slice is **In** for crypto/encoding scaffolding; zero-copy borrow remains residual, not silent green.

## What we do **not** claim

- Native `&str` borrow semantics (no lifetime / no shared slice type)
- Real `Byte` primitive distinct from `Int` (range typecheck, u8 ABI)
- Byte arrays (`List[Byte]`, fixed `[Byte; N]`, `&[Byte]`)
- Elimination of string value-copy overhead via true borrow
- DESIGN-done for pure cryptography/encoding on zero-copy byte buffers

## Path A residual floor (alpha)

**In (path A product free names + honesty rails):**

- Free names: `byte_at`, `bytes_len`, `byte_slice`, `bytes_eq` (CHS runtime + C emit)
- Owned byte-indexed slice as new `String` (`OoStr` copy) — useful for crypto/encoding without faking lifetimes
- `std/byte.oo` Int-backed 0..255 helpers + thin wrappers (`byte_get` / `byte_len` / `byte_sub` / `byte_eq`)
- Check remains fail-closed for unknown free names; no half-baked native `&str` type

**Still residual (not this pack):**

- Native `&str` with lifetimes / shared non-owning views
- True `Byte` / `List[Byte]` ABI in typecheck / Backend-C / LLVM
- Zero-copy string/byte views without retain-release of payload

## Rails

- Doc marker: `BYTE_STR_RESIDUAL_ALPHA`
- Residual smoke: `scripts/byte_str_residual_smoke.sh`
- Path A smoke: `scripts/byte_str_path_a_smoke.sh`
- Fixture residual: `fixtures/byte_str_marker.oo` (marker comment only)
- Fixture path A: `fixtures/byte_at_main.oo`, `fixtures/byte_slice_main.oo`
- Optional floor: `std/byte.oo` (Int convention + path A wrappers; residual documented in-file)

## Next (not claimed here)

Native borrow type + Byte / `List[Byte]` ABI in typecheck / Backend-C / LLVM; pure crypto on zero-copy byte buffers.
