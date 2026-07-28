## v0.17.0-alpha — WASM `if` lowering + outline `&&` fix + version alignment

### Round-7 Highlights

1. **WASM `if / then / else` lowering** *(DESIGN pillar: dual engine;
   first principles: the dual-engine claim is interpreter + LLVM +
   WASM; power law: this is the most visible asymmetry in the
   current state)*. LLVM had been lowering `if` expressions since
   v0.9.0-alpha but WASM rejected every program with a conditional.
   Added `emit_if` that produces structurally valid WAT:

   ```wat
   (if (result i64)
     cond_i64; i64.const 0; i64.ne;       ;; i64 -> i32 cond
     (then <then-wat>)
     (else <else-wat>))
   ```

   No-else branches fall back to `(else { i64.const 0 })` so the
   stack effect matches. `examples/conditional_add.oo` now compiles
   to both `.ll` and `.wat`. Nested `if`s and if-without-else also
   lower correctly.

2. **Outline `&&-ampersand` fix** *(DESIGN pillar: AI vibe-coding;
   first principles: the outline is the AI-facing API and must be
   syntactically correct; power law: every AI-facing output for
   capability params was wrong)*. `format_type` was returning
   `&NetCap` for capability types while `format_param` separately
   prepends `&` based on `is_ref`, producing `&&NetCap`. `format_type`
   now returns just `NetCap`; `is_ref` is the sole owner of the
   leading `&`. Verified on `security_cap.oo`:

   ```
   pub fn fetch_user_profile(net: &NetCap, user_id: Int) -> ...
   pub fn log_event(fs: &FsCap, message: String) -> ...
   ```

3. **Version alignment to v0.17.0-alpha** *(first principles: one
   version per project; power law: every consumer / install pin
   was reading a different number)*. Cargo.toml/binary was at
   `v0.16.0-alpha`, README at `v0.15.0-alpha`, qa and docs at
   `v0.15.0-alpha`. All artifacts now agree at `v0.17.0-alpha`.
   The codegen_wasm.rs WAT header comment (printed into every
   emitted `.wat` file) was also bumped from a stale v0.13.0-alpha
   string to v0.17.0-alpha.

### Verified

- **37/37 unit tests pass** (3 new WAT + 1 new outline regression)
- **130/130 real QA tests pass** (`openooda-qa/qa_runner.sh`)
- `ooda build --target wasm` now accepts `if` programs
- `ooda outline` outputs `net: &NetCap` (no `&&`)
- All artifacts report `v0.17.0-alpha`

### Earlier (kept)

- Runtime capability gate, sealed effect table, AST spans
- Must-use `Result` / `Option`, `let` immutability, `let mut` assign
- Real SHA-256 / HMAC / JSON via `sha2`, `hmac`, `serde_json`
- Real `std::thread` async via `async_spawn_internal` /
  `async_join_internal`
- Integer-subset LLVM IR + WAT backends with structural validation
- `ooda bench` per-proof verdicts, exits non-zero on failure
- Refinement type bounds checking (`Int[1..65535]` etc.)
- Atomic patch validation (parse + capability + type)

### Install

```bash
tar xzf ooda-v0.17.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.17.0-alpha-linux-x86_64:$PATH"
ooda --version   # 0.17.0-alpha
```

### Honest gaps remaining

- WASM subset does not yet lower `Match` / Float / String params
  / method-style calls — `bail!`s with a pointer to `ooda run`.
- `python_embed_internal` returns `Err("not implemented…")`.
- `lsp`, `pkg --install`, `migrate`, `replay` are scaffolding that
  `bail!` with clear messages.
- No "0ms GC pauses" — there is no GC.