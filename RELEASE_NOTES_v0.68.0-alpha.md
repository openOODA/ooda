## v0.68.0-alpha — FsCap file_size Lowering & Option is_some/is_none C Codegen

### Shipper
Antigravity (Google DeepMind) — openOODA rotation pass.

### Top-5 this rotation

1. **`FsCap` Receiver Method Lowering (`.file_size`):** Added object-capability receiver method lowering for `FsCap` (`fs.file_size(path)`) in `codegen_c.rs` and `chs_rt.c`, returning `Ty::Int` file size in native compiled C binaries (`ooda build`).
2. **Option Method Direct Lowering (`.is_some()`, `.is_none()`):** Added direct struct field lowering for `.is_some` and `.is_none` method calls in `codegen_c.rs`, eliminating pattern-matching overhead in native compiled C code.
3. **AI Diagnostic Immutable Assignment Codemod Patches:** Enriched `--json-errors` diagnostic payload for `AssignToImmutable` errors with structured machine-readable JSON code patch suggestions (`codemod`).
4. **Energy-Maneuverability Optimization:** Optimized zero-copy string slice allocation in `chs_rt.c` to use pointer offsets (`(OoStr){ .data = s.data + start, .len = len }`), cutting heap memory allocations ($W \to 0$) and increasing execution velocity ($V \uparrow$).
5. **Release Alignment & Synchronization:** Forward version bump to **v0.68.0-alpha** across Cargo, CLI, standard library, documentation, installer scripts, and public GitHub Pages website.

### Pin
v0.68.0-alpha

### Not claimed
Full CPython embedded runtime, zero-`.rs` self-hosting compiler, WASM capability IO support.
