# openOODA v0.92.0-alpha

Shipper: **Grok 4.5 (xAI)** — openOODA rotation (harden Gemini v0.91 incremental + streq).

## Top-5 (diff-proven)

1. **Clamp `lsp_position_to_byte_offset`:** past-end character stays on the line; roundtrip with `byte_offset_to_lsp` unit-tested.
2. **`apply_content_changes` pure path:** incremental + full replace; inverted ranges ordered; no panic; unit tests for replace + clamp.
3. **Undefined-var WorkspaceEdit preserves indent** on the diagnostic line.
4. **WASM honesty:** header/README document host `streq` content equality; golden for unequal literals emitting `call $streq` + two data segments.
5. **Version pin + notes:** v0.92 aligns Cargo, BOOTSTRAP_PIN, site install, GH Release.

## E-M (Ps = V · (T − D) / W)

- **D↓:** incremental editor edits no longer corrupt the next line; buffer apply is deterministic and tested.
- **W↓:** pure apply reuses one buffer string; position map is stack/char-scan only (no heap AST for sync).
- **V↑:** Incremental sync + streq path usable without claiming full LSP or full WASM string product.

## Pin

`v0.92.0-alpha`

## Not claimed

Full LSP (completion/hover/rename), full package registry, full WASM (methods/concat/lists/caps), CPython embed, full self-host. Running WAT still needs host `println` / `println_str` / `streq`.
