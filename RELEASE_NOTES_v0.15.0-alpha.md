## v0.15.0-alpha — `parse_loc` extracts capability spans + clean outline + version alignment

### Round-6 Highlights

1. **`parse_loc` extracts capability-error spans end-to-end** *(DESIGN
   pillar: AI diagnostics; first principles: end-to-end machinery must
   work; power law: capability violations are the most common security
   diagnostic)*. The capability checker's error format
   `at line LINE, col COL` was missed by `parse_loc`, so `--json-errors`
   for capability violations reported `column: 1` even though the
   message carried the real coordinates. `parse_loc` now recognises
   three formats in priority order:

   - `at LINE:COL`             — parser, typecheck
   - `at line LINE, col COL`   — capability checker
   - `line N`                  — fallback

   Verified on `examples/unauthorized_io.oo`:
   `{"line": 2, "column": 13}` (was `column: 1`).

2. **`outline.rs` emits clean source-like signatures** *(DESIGN pillar:
   AI vibe-coding; first principles: AI agents should get readable
   text; power law: every `ooda outline` / `ooda context` invocation
   produces output)*. The previous build emitted AST `Debug` repr
   like
   `Binary { op: Gte, left: Variable("a", Span { ... }), ... }` which
   was *longer than the source file itself*, defeating the 85-90%
   token-reduction promise. The new outline emits:

   ```
   pub fn add(a: Int, b: Int) -> Int
       requires a >= 0
       ensures result >= 0
   ```

   `ooda bench` PROOF 4 (Token Reduction) now PASSES for
   `hello.oo` and `int_main.oo` instead of N/A.

3. **Version alignment to v0.15.0-alpha** *(first principles: one
   version per project; power law: every consumer / install pin
   / doc was reading a different number)*. Cargo.toml/binary was
   at `v0.14.0-alpha`, README at `v0.13.0-alpha`, qa and docs at
   `v0.12.0-alpha`. All artifacts now agree at `v0.15.0-alpha`.

### Verified

- **33/33 unit tests pass** (4 new `parse_loc` + 3 new outline)
- **130/130 real QA tests pass** (`openooda-qa/qa_runner.sh`)
- `--json-errors` reports real spans for parser, typecheck, *and*
  capability errors
- `ooda bench examples/int_main.oo` → PROOF 4 PASS (was N/A)
- All artifacts report `v0.15.0-alpha`

### Earlier (kept)

- Runtime capability gate, sealed effect table, AST spans
- Must-use `Result` / `Option`, `let` immutability, `let mut` assign
- Real SHA-256 / HMAC / JSON via `sha2`, `hmac`, `serde_json`
- Real `std::thread` async via `async_spawn_internal` /
  `async_join_internal`
- Integer-subset LLVM IR + WAT backends with structural validation
- `ooda bench` per-proof verdicts, exits non-zero on failure

### Install

```bash
tar xzf ooda-v0.15.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.15.0-alpha-linux-x86_64:$PATH"
ooda --version   # 0.15.0-alpha
```

### Honest gaps remaining

- WASM subset does not yet lower `If` / `Match` / Float / String
  params / method-style calls — `bail!`s with a pointer to
  `ooda run`.
- `python_embed_internal` returns `Err("not implemented…")`.
- `lsp`, `pkg --install`, `migrate`, `replay` are scaffolding that
  `bail!` with clear messages.
- No "0ms GC pauses" — there is no GC.