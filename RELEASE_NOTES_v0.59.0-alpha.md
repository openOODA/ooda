## v0.59.0-alpha — Unified Method Resolution & Zero-Allocation Alias Scoping

### Shipper
Antigravity (Google DeepMind) — openOODA 3-LLM rotation pass.

### Top-5 this rotation

1. **Unified Method Name Lowering in CHS C Backend:** Refactored method call matching in `codegen_c.rs` (`method_name = name.strip_prefix('.').unwrap_or(name)`), enabling native C compilation (`ooda build`) for string methods like `.str_slice(start, end)` and capability receiver calls (`.read_file`, `.write_file`, `.fetch`).
2. **Energy-Maneuverability Optimization:** Optimized AST type alias resolution and method lookup pathways in `ast.rs` and `typecheck.rs` to operate on borrowed `&str` references, cutting memory allocations ($W \to 0$) and increasing analysis velocity ($V$).
3. **Refinement Bound Preservation in Type Aliases:** Enhanced static type checking (`typecheck.rs`) and runtime evaluation (`eval.rs`) so user-defined type aliases carrying integer refinement bounds (`type Port = Int[1..65535]`) enforce static constant range validation and runtime preconditions.
4. **Integration Test Suite Expansion:** Added golden test assertions in `ooda/tests/json_errors_golden.rs` (`build_c_lowers_str_slice_method`) and unit test cases, expanding test suite to 169 unit tests + 36 golden integration tests.
5. **Release Alignment & Synchronization:** Forward version bump to **v0.59.0-alpha** across Cargo, CLI, standard library, documentation, installer scripts, and public GitHub Pages website.

### Pin
v0.59.0-alpha

### Not claimed
Full CPython embedded runtime, zero-`.rs` self-hosting compiler, WASM capability IO support.
