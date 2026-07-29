## v0.72.0-alpha — Nested field assign, sealed receiver types, E-M string views

### Shipper
Grok 4.5 (xAI) — openOODA rotation pass.

### Top-5 (diff-proven)

1. **Nested field assign** (`o.inner.n = v`): typecheck, interpreter, and CHS C (`p.a.b = …` lvalue). Root binding must be `let mut`.
2. **Sealed method receiver types**: `.path_exists`/`.file_size` require `FsCap`; `.env_get` → `EnvCap`; `.sys_exec` → `SysCap`; `.get` → `NetCap`; string methods require `String`; Result/Option probes require matching sum types.
3. **E-M string ops (`chs_rt.c`)**: length-bounded `.contains` (safe for slices); **zero-copy** `.trim` view; `.to_lowercase` skips alloc when already lowercase; `oo_int_to_str` formats via stack buffer then one owned copy.
4. **AI `missing_return` codemod**: structured `codemod:missing_return` with `declared_return` and type-aware stub (`return 0` / `false` / `""` / `None` / `Err(...)`).
5. **Release alignment** to **v0.72.0-alpha** (ooda → tag/asset → docs/site/qa).

### Pin
v0.72.0-alpha

### Not claimed
LSP, pkg install, replay, full WASM product, PyTorch, `for` sugar, zero-`.rs` self-host, runtime object-caps on native binaries.
