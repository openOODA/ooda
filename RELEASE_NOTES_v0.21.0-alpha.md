# openOODA v0.21.0-alpha — M0 CHS host surface

## Milestone

**M0** of the self-host escape plan (strategy D′): grow the Compiler Host Subset
language surface in stage-0 so a real `oodac` in `.oo` becomes possible.

`DESIGN.md` is unchanged.

## Landed (interpreter host)

| Feature | Notes |
|---|---|
| Real `read_file` / `write_file` / `fs_read` / `fs_write` | Disk I/O under sealed `&FsCap`; honest `Result` / `Err` |
| Cap deny without `&FsCap` | Static + runtime default-deny |
| `List[T]` | `list_new`, `list_push`, `list_get`, `list_len`, `.len` |
| String walk | `chars_len`, `char_at`, `str_slice`, `char_is_{digit,alpha,space}` |
| Named structs | `type T = struct { … };` + `T { f: e }` + field access `t.f` |
| Argv | `ooda run file.oo -- args…` → `main(args: List[String], …)` |
| `env_get` | Real env lookup under `&EnvCap` |

## Examples

- `examples/chs_fs_roundtrip.oo`
- `examples/chs_list_string.oo`
- `examples/chs_struct_token.oo`
- `examples/chs_token_walk.oo` — integration golden (FS + List + string + argv)

## Documented host-only (LLVM kill date M4)

List, String walk, struct, and FsCap I/O are **not** lowered by the integer-subset
LLVM backend. `ooda build` rejects them with a clear error. See `bootstrap/CHS.md`.

## Still not implemented (fail non-zero)

LSP, pkg install, migrate, replay, full WASM product, PyTorch, `for` sugar,
self-host / fixed-point, LLVM CHS emit.

## std

`openooda-std/src/fs.oo` no longer returns fake `"file content"`; documents
forwarding to stage-0 sealed builtins.
