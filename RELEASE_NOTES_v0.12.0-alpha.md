## v0.12.0-alpha — WASM no longer lies + real spans in `--json-errors`

### Round-5 Highlights

1. **WASM codegen fails non-zero on its own gaps** *(first principles:
   "unfinished features must fail non-zero")*. The previous
   `codegen_wasm.rs` emitted `local.get $res` for an undeclared
   local `$res` while the CLI printed "⚡ Successfully compiled
   WebAssembly module." The new codegen:
   - Emits `(local $name i64)` + `local.set $name` for `let` bindings.
   - `bail!`s with a clear "use `ooda run`" message when it encounters
     `println` (String println can't lower to the i64 host import),
     capability parameters, String/Float types, `If` / `Match`
     expressions, or method-style calls.
   - Runs a structural pass that flags undeclared locals / missing
     `return` instructions and (when `wasm-tools` is on PATH)
     round-trips through `wasm-tools validate`.
   - Five new unit tests in `codegen_wasm::tests`.

2. **`--json-errors` reports real source spans** *(first principles:
   AI diagnostics need the actual location to be surgically useful;
   power law: the JSON machinery was wired but fed garbage `line:1,
   column:1` for every typecheck error)*.
   - New `Span { line, col }` carried on every `Expression`,
     `Statement`, and `FunctionDecl`.
   - Parser populates it from `SpannedToken`.
   - `typecheck.rs` includes the span in every error message.
   - `AiDiagnostic` surfaces it via the existing `parse_loc` helper.
   - New test: `typecheck::tests::type_error_includes_real_source_span`.

3. **Version alignment to v0.12.0-alpha** *(first principles: one
   version per project)*. The previous round left `Cargo.toml` /
   `dist/ooda` at `v0.10.1-alpha` while README / qa / docs all said
   `v0.10.0-alpha`, and **no v0.10.1-alpha release existed on GitHub**
   (latest was still `v0.11.0-alpha`). This round bumps all artifacts
   forward to `v0.12.0-alpha` and publishes the matching GitHub
   release so the install pin is real.

### Verified

- **22/22 unit tests pass** (5 new WASM + 1 new typecheck span test)
- **130/130 real QA tests pass** (`openooda-qa/qa_runner.sh`)
- `ooda build --target wasm` on a pure-int program emits structurally
  valid WAT
- `ooda build --target wasm` on a program with `println`/`String`
  exits 1 with a clear error message

### Earlier (kept)

- **Runtime capability enforcement** — interpreter consults the
  sealed `EFFECT_BUILTINS` table at call time.
- **Real SHA-256 / HMAC / JSON** via `sha2`, `hmac`, `serde_json`.
- **Real `std::thread` async** via `async_spawn_internal` /
  `async_join_internal`.
- **Honest `ooda bench`** — per-proof verdicts, exits non-zero on
  failure.
- **Integer-subset LLVM IR backend** with structural validation and
  optional `llvm-as` round-trip.

### Install

```bash
tar xzf ooda-v0.12.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.12.0-alpha-linux-x86_64:$PATH"
ooda --version   # 0.12.0-alpha
```

### Honest gaps remaining

- WASM subset does not yet lower `If` / `Match` / Float / String /
  method-style calls — `bail!`s with a pointer to `ooda run`.
- `python_embed_internal` returns `Err("not implemented…")`.
- `lsp`, `pkg --install`, `migrate`, `replay` are scaffolding that
  `bail!` with clear messages.
- No "0ms GC pauses" — there is no GC.