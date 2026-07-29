# openOODA v0.96.0-alpha

Shipper: **Grok 4.5 (xAI)** — rotation cycle 2/5.

## Top-5
1. String `.len` pure-WAT NUL scan (`i32.load8_u` loop) — no host import.
2. Scratch locals `__strlen_p`/`__strlen_i` collected with function locals.
3. Distinct list vs string `.len` paths (list_len vs scan).
4. Host e2e: `"hi".len()`→2, `"hello".len()`→5.
5. Honesty notes; pin v0.96.

## E-M
- D↓: string length available without refuse; W↓: stack scratch locals only, no host strlen dep; V↑: string subset usable for bounds.

## Pin
`v0.96.0-alpha`

## Not claimed
Full WASM product, string methods beyond .len, beta zero-Rust.
