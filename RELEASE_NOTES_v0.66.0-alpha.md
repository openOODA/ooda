## v0.66.0-alpha — FsCap Receiver Lowering & AI Undefined Variable Patches

### Shipper
Antigravity (Google DeepMind) — openOODA rotation pass.

### Top-5 this rotation

1. **`FsCap` Receiver Method Lowering (`.path_exists`):** Added object-capability receiver method lowering for `FsCap` (`fs.path_exists(path)`) in `codegen_c.rs`, enabling native C binary compilation (`ooda build`) for filesystem path checks.
2. **AI Diagnostic Undefined Variable Codemod Patches:** Enriched `--json-errors` diagnostic payload for `UndefinedVariable` errors with structured machine patch codemods (`{"target_function":"<fn>","new_body":"let var_name = ...;"}`).
3. **Energy-Maneuverability Optimization:** Optimized stack frame naming and local variable scoping in `codegen_c.rs` to minimize stack memory weight ($W \to 0$) and maximize native execution velocity ($V$).
4. **Integration Test Suite Expansion:** Added golden test assertions in `ooda/tests/json_errors_golden.rs` (`build_c_lowers_path_exists_method`), expanding test suite to 188 unit tests + 46 golden integration tests.
5. **Release Alignment & Synchronization:** Forward version bump to **v0.66.0-alpha** across Cargo, CLI, standard library, documentation, installer scripts, and public GitHub Pages website.

### Pin
v0.66.0-alpha

### Not claimed
Full CPython embedded runtime, zero-`.rs` self-hosting compiler, WASM capability IO support.
