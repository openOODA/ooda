## v0.29.0-alpha — Real `ooda migrate` + `--json-errors` telemetry + `old()` validation

### Round-9 Highlights (E-M formula: `Ps = V * (T - D) / W`)

1. **Real `ooda migrate` codemod** *(DESIGN §17.6 promise, "ready for use"
   criterion; ↑T — users can now upgrade v0.10-era code)*. `src/migrate.rs`
   was a 17-line refusal stub. It now:
   - Parses the .oo file (fails closed on syntax errors).
   - Walks the AST finding non-exhaustive match expressions on
     Result/Option.
   - Locates the matching close-brace via a tolerant text scan
     (skips strings and comments).
   - Replaces an existing trailing `,` with the wildcard arm
     (or inserts the arm if no trailing comma).
   - Inserts `, _ => process_exit(1),` so the typecheck passes
     while the user gets a loud runtime signal at the
     previously-unhandled variant.
   - 8 new unit tests cover rbrace scanner + integration.
   - `ooda migrate --edition 1999` fails closed (only `2026` supported).

2. **`--json-errors` carries real timing telemetry** *(DESIGN pillar:
   AI diagnostics + E-M; ↑V via faster agent loops; ↓D via
   no hardcoded numbers)*. `with_timings()` was dead code.
   `load_and_analyze` now uses `Instant::now()` to time
   `parse_us`, `capability_us`, `typecheck_us`, and the failure
   paths attach them via `.with_timings()`. Output verified:
   ```json
   "timings_us": { "parse_us": 84, "check_us": 35 }
   ```
   A `tests/json_errors_golden.rs` integration test rejects
   hardcoded `em_savings` strings.

3. **`old(x)` postcondition validated at compile time, snapshot
   skipped at runtime when unused** *(DESIGN pillar: contracts; E-M,
   ↓D + ↓W)*. `src/typecheck.rs` Call arm: when `name == "old"`,
   the first arg must be a `Variable` in the enclosing env, with
   a specific error otherwise. `src/ast.rs` adds
   `FunctionDecl::uses_old_state()` that recursively walks body +
   ensures + verify block. `src/eval.rs` only allocates the
   snapshot HashMap when `uses_old_state()` is true — zero bytes
   touched for the common case where contracts don't reach for
   prior state.

4. **`examples/old_state.oo` (.oo format)** — exercises `old()` with
   `requires` + `ensures` + `verify`. The new
   `tests_behavior/beh_08_old_state.oo` locks it in. (Bumped from
   137 to 138 QA tests.)

5. **Web page updated for v0.29.0-alpha** with an `old_state.oo`
   dropdown entry, real captured CLI output, and a fixed stale
   `v0.13.0-alpha and later` reference in the must_use example
   comment.

### Earlier (kept)

- Runtime capability gate, sealed effect table, AST spans
- Must-use `Result` / `Option`, `let` immutability, `let mut` assign
- Integer-subset LLVM IR + WAT backends with structural validation
- CHS C backend (gcc + `runtime/chs_rt.c`, no clang required)
- `oodac` self-host, real 2-generation fixed-point referee
- Real SHA-256 / HMAC / JSON via `sha2`, `hmac`, `serde_json`
- Real `std::thread` async
- Refinement type bounds (`Int[lo..hi]`)
- Atomic patch validation (parse + capability + type)
- `ooda bench` per-proof verdicts, exits non-zero on failure
- 5 unit tests enforce version-consistency across all 5 artifacts

### Verified

- **84/84 unit tests pass** (74 lib + 9 bin + 1 parse_loc)
- **138/138 real QA tests pass** (`openooda-qa/qa_runner.sh`)
- All 6 artifacts (Cargo.toml, binary, README.md, qa/README.md,
  docs/index.html, scripts/release.sh) report `v0.29.0-alpha`
- `cargo test version_consistency` passes (5 version tests)
- Independent agent review confirmed all 7 validation tasks pass

### Install

```bash
tar xzf ooda-v0.29.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.29.0-alpha-linux-x86_64/bin:$PATH"
ooda --version   # 0.29.0-alpha
```

### Honest gaps remaining

- WASM `Match` / Float params / String params / method calls still
  `bail!` with a pointer to `ooda run`.
- `python_embed_internal` returns honest `Err("not implemented…")`.
- `lsp`, `pkg --install`, `migrate` (now partial!), `replay` are
  scaffolding that `bail!` with clear messages.
- No "0ms GC pauses" — there is no GC.