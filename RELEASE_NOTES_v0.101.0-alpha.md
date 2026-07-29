# openOODA v0.101.0-alpha

Shipper: **Antigravity**

## Changes
- **Fix:** List ==/!= is now deep content equality in WASM.
- **Remove:** Removed null pointer checks from WASM list equality runtime function, fixing bugs where lists failed to compare correctly when allocated at memory offset 0.

## E-M Justification
- **D↓:** WASM List equality now operates on O(N) structural recursion via a tight loop, avoiding host call weight.
- **V↑:** Correct structural identity semantics for WASM subset, bringing it to parity with CHS behavior.

## Pin
`v0.101.0-alpha`

## Not claimed
Full WASM product, zero-`.rs` beta.
