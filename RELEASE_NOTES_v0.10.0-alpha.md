## v0.10.0-alpha — Honest bench verdicts + version alignment + stale-artifact purge

### Round-4 Highlights (this push)

1. **Honest `ooda bench`** — `src/bench.rs` now records a per-proof
   verdict (`VERIFIED` / `TRAP FIRED` / `NOT APPLICABLE` / `FAILED`) and
   prints a truthful summary. The previous build printed
   "🏆 ALL EMPIRICAL CLAIMS VERIFIED SUCCESSFULLY" unconditionally,
   even on examples like `unauthorized_io.oo` where the capability
   check correctly fired as a trap. PROOF 4 (token reduction) is now
   informational since the AST-debug outline format is sometimes
   longer than the source. `ooda bench` exits non-zero when any proof
   fails.

2. **Version alignment** — `Cargo.toml`, `src/main.rs` (clap),
   `scripts/release.sh`, `README.md`, `dist/ooda`, `qa/README.md`, and
   `docs/index.html` are all bumped to **v0.10.0-alpha**. Before this
   round, the README claimed `v0.11.0-alpha`, the binary reported
   `v0.9.1-alpha`, and the qa/docs sites reported `v0.9.0-alpha`.
   Every consumer of the project now sees a single version.

3. **Stale-artifact purge** —
   - Removed `examples/hello.wasm` (was a 185-byte hardcoded WAT
     template, not real WASM; the CLI rejects `--target wasm` with
     a clear error).
   - Removed `ooda-v0.1.1-alpha-linux-x86_64.tar.gz` and
     `ooda-v0.9.0-alpha-linux-x86_64.tar.gz`.
   - Moved `examples/self_hosted_*.oo` to `examples/prototypes/`
     with a README explaining they are illustrative only, not a
     self-hosted compiler.

### Earlier in v0.10.0-alpha (kept)

- **Runtime capability enforcement** — the interpreter consults the
  sealed `EFFECT_BUILTINS` table at call time and refuses sealed
  effects unless the enclosing function declared the matching cap.
- **Real async** — `async_spawn_internal` / `async_join_internal`
  spawn and join real `std::thread` handles.
- **Real SHA-256 / HMAC-SHA256 / JSON** via the `sha2`, `hmac`, and
  `serde_json` crates (verified against RFC test vectors).
- **Spans in lexer** — tokens carry `line:col`; `--json-errors`
  reports real spans (no more always `1:1`).
- **Real `patch` / `context` / `fmt`** — `patch` validates a
  replacement body via the parser before writing; `context` builds
  real outline/reflect JSON within a tier budget; `fmt` no longer
  overwrites the source with `Debug` AST.
- **Honest stub CLI commands** — `lsp` / `pkg --install` / `migrate`
  / `replay` / `--target wasm` all `bail!` with a clear "not
  implemented" message instead of fake green output.
- **Integer-subset LLVM IR backend** with `if` expression lowering,
  structural validation (duplicate `ret`, undefined `#0`, type
  mismatches), and optional `llvm-as` round-trip validation.

### Tests

- 16/16 unit tests pass.
- 129/129 real QA tests pass (`openooda-qa/qa_runner.sh`).
- `ooda bench` exits non-zero on genuine regressions and reports
  trap firings as informational rather than success.

### Install

```bash
tar xzf ooda-v0.10.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.10.0-alpha-linux-x86_64:$PATH"
ooda --version   # 0.10.0-alpha
```

### Honest gaps remaining

* `python_embed_internal` returns `Err("not implemented…")`.
* WASM target is not implemented.
* `lsp`, `pkg --install`, `migrate`, `replay` are scaffolding that
  bail with clear messages.
* No "0ms GC pauses" — there is no GC; the interpreter uses
  `Box`/`HashMap`.