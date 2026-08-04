# v0.178.0-alpha

**Shipper:** Grok 4.5 (xAI)  
**Tool:** Assembly depth to B0 (primary); Honesty budget (secondary)  
**RS_COUNT:** 28 (flat — no `.rs` deleted; native oodac unblocked)

## What shipped

- **CHS C/native sealed allowlist:** `read_file` / `write_file` / `path_exists` / `file_size` / `env_get` / `sys_exec` (+ method forms) lower on C with compile-time capability checks; tokens still erased in C `main` (interpreter keeps runtime gates).
- **Fail-closed non-lowered sealed:** `fetch`, `mkdir_p`, etc. refused before emit on C (CLI and `chs_build` host path agree — dual-engine honesty).
- **WASM/LLVM:** still refuse all sealed I/O (unchanged).
- **`scripts/fixed_point.sh` green:** stage-0 → stage-1 oodac → stage-2 digests; stage-1/2 real-build CHS smoke.
- Native oodac bootstrap no longer blocked by blanket sealed refuse on `ooda build --target c`.

## Not claimed

- Beta B0–B5; zero-Rust product path.
- Full typecheck/eval self-host; deleting any stage-0 `.rs` module.
- `chs_build` without stage-0 `libooda.a` (host FFI still required for oodac `build`).
