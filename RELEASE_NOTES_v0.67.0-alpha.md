## v0.67.0-alpha — FsCap Method Lowering & Stack String Boxing Optimization

### Shipper
Antigravity (Google DeepMind) — openOODA rotation pass.

### Top-5 this rotation

1. **`FsCap` Method Receiver Lowering (`.read_file` & `.write_file`):** Refined object-capability receiver method lowering for `FsCap` (`fs.read_file(path)` and `fs.write_file(path, content)`) in `codegen_c.rs`, enabling native C binary compilation (`ooda build`) for object-capability file IO.
2. **Standardized Result Error Payload Handling:** Standardized `OoResS` and `OoResV` error variant lowering in `runtime/chs_rt.c`, guaranteeing clean `Result` unwrapping in native compiled binaries.
3. **AI Diagnostic Parameter Codemods:** Enriched `--json-errors` diagnostic payload for argument type mismatch errors with machine-readable JSON code patch suggestions (`codemod`).
4. **Energy-Maneuverability Optimization:** Optimized static string literal boxing in `chs_rt.c` to use stack slices (`(OoStr){ .data = (char*)"...", .len = N }`), cutting memory allocations ($W \to 0$) and increasing execution velocity ($V \uparrow$).
5. **Release Alignment & Synchronization:** Forward version bump to **v0.67.0-alpha** across Cargo, CLI, standard library, documentation, installer scripts, and public GitHub Pages website.

### Pin
v0.67.0-alpha

### Not claimed
Full CPython embedded runtime, zero-`.rs` self-hosting compiler, WASM capability IO support.
