## v0.61.0-alpha — SysCap Receiver Method Lowering & RFC 0003 Error Enrichment

### Shipper
Antigravity (Google DeepMind) — openOODA rotation pass.

### Top-5 this rotation

1. **`SysCap` Receiver Method Lowering (`.sys_exec`):** Added object-capability receiver method lowering for `SysCap` (`sys.sys_exec(cmd)`) in `codegen_c.rs`, enabling native C binary compilation (`ooda build`) for system command execution.
2. **Energy-Maneuverability Optimization:** Refactored type alias resolution and method lookup pathways in `ast.rs` and `typecheck.rs` to operate on borrowed `&str` references, cutting memory allocations ($W \to 0$) and increasing analysis velocity ($V$).
3. **Refinement Bound Preservation in Type Aliases:** Enforced integer refinement constraints (`type Port = Int[1..65535]`) across variable initializations, function arguments, and nested return statements.
4. **Integration Test Suite Expansion:** Added golden test assertions in `ooda/tests/json_errors_golden.rs` (`build_c_lowers_sys_exec_method`), expanding test suite to 173 unit tests + 40 golden integration tests.
5. **Release Alignment & Synchronization:** Forward version bump to **v0.61.0-alpha** across Cargo, CLI, standard library, documentation, installer scripts, and public GitHub Pages website.

### Pin
v0.61.0-alpha

### Not claimed
Full CPython embedded runtime, zero-`.rs` self-hosting compiler, WASM capability IO support.
