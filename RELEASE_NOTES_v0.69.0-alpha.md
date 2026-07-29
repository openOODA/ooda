## v0.69.0-alpha — EnvCap `.env_get` (interp) + dual-engine seal honesty

### Shipper
Honesty pass after Antigravity rotation review (Grok) — openOODA.

### What actually changed (diff-proven)

1. **`EnvCap` `.env_get` on the interpreter:** `env.env_get(key)` returns `Result[String, String]` under a live `&EnvCap` (`ooda run`). Missing keys yield `Err`.
2. **Dual-engine seal table closed:** method forms `.path_exists`, `.file_size`, `.sys_exec` (and free `file_size`) are now sealed like free/` .read_file` / `.env_get`. `ooda build` for C/LLVM/WASM/native **refuses** sealed I/O (no ambient native bypass via unsealed method names).
3. **C Option/Result probes:** `.is_some` / `.is_none` / `.is_ok` / `.is_err` lower to `OoResS.ok` (shared layout). Pure Result probes without sealed I/O still build on C.
4. **C cap tokens:** marked `/*cap*/` in the emitter (compile-only placeholders; **no runtime gate on C** — refuse sealed programs instead).
5. **Version pin** to **v0.69.0-alpha** in Cargo, CLI, install pin, and monorepo siblings when shipping.

### Explicit non-claims (this tag)

- Native C/LLVM **does not** lower sealed env/FS/sys with runtime capability tokens.
- No new MissingReturn / AssignToImmutable codemod in this tag.
- No measured snprintf / zero-copy E-M “win” in `chs_rt.c` this tag.
- C `oo_env_get` / FS helpers remain host runtime for **non-sealed** or host paths only; user programs with sealed effects must use `ooda run`.

### Pin
v0.69.0-alpha

### Not claimed
Full CPython embedded runtime, zero-`.rs` self-hosting compiler, WASM capability IO support, runtime object-caps on native binaries.
