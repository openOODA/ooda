## v0.26.0-alpha — Honest net, fail-closed refinements, real AI diagnostic goldens

### Rotation highlights (DESIGN pillars)

1. **Capabilities — sealed net is real**  
   `fetch` / `http_get` / `net_get` / `downloadData` / `.get` now perform HTTPS GET via curl under `&NetCap`, returning `Result[String, String]`.  
   `std::net` no longer fakes `Ok("200 OK")` — it calls `fetch`.  
   Deny path still traps without a token (static + runtime).

2. **Contracts / types — `where` fail-closed**  
   `type T = Int where …` is **rejected at parse** (was discarded → refinement theater).  
   Use `requires` / `ensures` or `Int[lo..hi]` annotations. QA + examples migrated.

3. **Honesty — fake greens killed**  
   - `beh_07_pytorch_model.oo` asserts `is_err()` (python bridge not implemented)  
   - `sec_01_forged_capability_trap.oo` actually calls `fetch` without `NetCap` (must trap)  
   - RFC auditor no longer soft-passes (`|| true` removed)  
   - `std::python` / `std::async` verify or `main` probes match runtime truth

4. **AI diagnostics — golden test**  
   `tests/json_errors_golden.rs` asserts `--json-errors` on `unauthorized_io.oo` yields  
   `CapabilitySecurityViolation`, non-zero exit, `line`, `message`, and non-empty `suggested_fix.diff`.

5. **Version pin alignment**  
   Canonical `v0.26.0-alpha` locked across Cargo, clap, README, `install/install.oo`,  
   `install/BOOTSTRAP_PIN`, website install defaults, docs/qa headers.  
   Dual-engine CLI help no longer claims full production LLVM for every `build`.

### QA

`./qa_runner.sh` — **136/136** with RFC auditor required non-zero on failure.

### Install

```bash
curl -fsSL https://openOODA.github.io/install | sh
# or
tar xzf ooda-v0.26.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.26.0-alpha-linux-x86_64/bin:$PATH"
ooda --version   # 0.26.0-alpha
```

### Not claimed (still fail closed)

LSP, pkg install, migrate, replay, full WASM product, AES-256, embedded CPython,  
type-alias `where` bound checking, full SPEC self-host.
