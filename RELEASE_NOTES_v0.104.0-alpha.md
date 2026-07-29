# openOODA v0.104.0-alpha
Shipper: **Grok 4.5 (xAI)** — cycle 3/5.
## Top-5
1. `.str_slice` bump-heap copy + host e2e → "ell"
2. Heap global without list RT when only slices needed
3. Unique temp dirs for wasm_host (parallel race fix)
4. Full parallel wasm_host suite green
5. Pin v0.104
## E-M
D↓ no flaky temp collisions; W↓ heap only when slice/list needs it; V↑ substring on WASM
## Pin
v0.104.0-alpha
