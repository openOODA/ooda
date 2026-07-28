## v0.24.0-alpha — WASM Float lowering + version-consistency test + stdlib QA

### Round-8 Highlights

1. **WASM `Float` expression lowering** *(DESIGN pillar: dual engine;
   first principles: the dual-engine claim requires both backends to
   support the same subset; power law: `--target wasm` was the only
   backend rejecting Float programs while the interpreter + LLVM +
   CHS C backends all accepted them)*. Round 7 added `if` lowering;
   `Float` was still rejected with "use `ooda run`". The codegen
   now uses a `BTreeMap<String, &'static str>` for *typed* locals
   (`i64` | `f64`) and emits `f64.const` / `f64.add` / `f64.sub` /
   `f64.mul` / `f64.div` / `f64.eq` / `f64.ne` / `f64.lt` / `f64.gt` /
   `f64.le` / `f64.ge` for Float arithmetic. Mixed Int+Float ops
   promote via `f64.convert_i64_s`; `println(f64)` truncates via
   `i64.trunc_f64_s`. Verified on `examples/float_main.oo`.

2. **Version-consistency tests** *(first principles: one version per
   project; power law: every consumer / install pin / doc was reading
   a different number)*. Rounds 6, 7, and 8 each had to manually
   re-align `Cargo.toml`, `src/main.rs` (clap version),
   `scripts/release.sh`, `README.md`, `qa/README.md`, and
   `docs/index.html`. Three new unit tests in `src/main.rs` now
   assert the canonical version (`v0.24.0-alpha`) appears in the
   clap string, the README header, and that `release.sh` derives
   from Cargo (no hardcoded drift). Any future bump that forgets
   an artifact will fail CI loudly.

3. **openooda-std `.oo` integration tests** *(DESIGN pillar: contracts
   + types; .oo-format rule; first principles: if it's not tested,
   it's not real)*. New `openooda-qa/tests_stdlib/` directory with
   pure `.oo` programs that import from `openooda-std` and verify
   real RFC vectors:

   ```ooda
   // crypto_real.oo
   assert_eq!(
       sha256_abc(),
       "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
   );
   ```

   `crypto_real.oo` checks SHA-256("abc"), SHA-256(""), and
   HMAC-SHA256("key", "The quick brown fox...") against the
   published RFC vectors. `json_real.oo` checks roundtrip + malformed
   → Err. A stdlib regression (wrong hash, off-by-one hex, swapped
   bytes) would fail loudly.

### Earlier (kept)

- Runtime capability gate, sealed effect table, AST spans
- Static type checker (must-use Result / let mut assign / refinement bounds)
- Integer-subset LLVM IR + WAT backends with structural validation
- CHS C backend (gcc + `runtime/chs_rt.c`, no clang required)
- `oodac` self-host fixed-point referee (real 2-generation compile)
- Real SHA-256 / HMAC / JSON via `sha2`, `hmac`, `serde_json`
- Real `std::thread` async

### QA Suite Phases (135/135 pass)

1. Behavioral feature suite
2. Negative fault injection (must trap)
3. Capability traps from the behavioral matrix
4. High-value targeted suite
5. AI UX / Bench / Concurrency / Cross-target / Fuzz / Security
6. Core Contract / Security / Type-system suite
7. WebAssembly target emitter (`.wat`) probe
7b. **WASM Float arithmetic emitter** (round 8)
7c. **CHS C backend native binary** (round 8; gcc + runtime)
7d. **CHS struct + Option + match** (round 8)
7e. **openooda-std `.oo` programs** (round 8; SHA-256 + JSON RFC vectors)
8. RFC compliance audit

### Install

```bash
tar xzf ooda-v0.24.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.24.0-alpha-linux-x86_64/bin:$PATH"
ooda --version   # 0.24.0-alpha
```

### Honest gaps remaining

* LSP, `pkg --install`, `migrate`, `replay` are scaffolding that
  `bail!` with clear messages.
* WASM `Match` lowering is still rejected (only Float + If + Int).
* `python_embed_internal` returns honest `Err`.
* No "0ms GC pauses" — there is no GC.