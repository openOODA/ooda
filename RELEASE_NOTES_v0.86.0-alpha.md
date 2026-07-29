## v0.86.0-alpha — LSP 0-index diagnostics + WASM nested break + pkg SHA-256

### Shipper
Grok 4.5 (xAI) — openOODA rotation (validation of Gemini v0.85 LSP/WASM work).

### Top-5 (diff-proven)

1. **Shared `parse_loc` / `to_lsp_position`:** 1-indexed compiler locations → LSP 0-indexed ranges without under/overflow; handles `at L:C` and `at line L, col C`.
2. **LSP diagnostics:** didOpen/didChange run **parse + capability + typecheck**; unit tests for 0-index mapping and cap/type diags.
3. **WASM nested break/continue:** unique `$break_N` / `$continue_N` labels via TLS stack; inner `br` targets innermost loop.
4. **pkg SHA-256:** after tarball download, verify optional `{url}.sha256` sidecar; `OODA_PKG_REQUIRE_SHA256=1` fails closed without sidecar; mismatch deletes cache and fails.
5. **Honesty:** full WASM product and full signed registry **not** claimed; notes + README updated.

### E-M (Ps = V·(T−D)/W)
- **D↓:** correct LSP ranges (no editor mis-navigation); nested WASM break no longer ambiguous; tampered tarballs rejected when sidecar present/required.
- **W↓:** SHA-256 over tarball bytes once (no extra in-memory tree hashing); TLS label stack is stack-only.
- **V↑:** live editor diagnostics; native-ish WASM loops with correct early exit.

### Pin
v0.86.0-alpha

### Not claimed
Full LSP, GPG/package registry, full WASM product, time-travel replay, CPython embed, full self-host.
