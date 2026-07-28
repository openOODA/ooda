# CHS_lang / OODA₀ — Compiler Host Subset (draft, freeze at M1)

**Status:** M0 stage-0 surface landed (interpreter host).  
**LLVM / native:** List, String, struct, FsCap remain **host-only** until progressive EMIT (kill date: M4).  
**Constitution:** `DESIGN.md` unchanged.

## Dual-engine done-definition

A feature is fully **in CHS** when the corpus passes `ooda run` **and** `ooda build` (native) **and** trap tests.  
Until EMIT catches up, features below are **CHS host surface** (usable for oodac.oo on the interpreter).

## Types

| Type | Status |
|---|---|
| `Int`, `Bool`, `String`, `Void` | host + (Int/Bool/Float LLVM subset) |
| `Option[T]`, `Result[T, E]` | host |
| `List[T]` | host (`list_new`, `list_push`, `list_get`, `list_len`, `.len`) |
| `struct` via `type Name = struct { fields }` | host; field access `t.field` |
| `&FsCap`, `&EnvCap`, … | host tokens; real FS/env builtins |

## Control

`fn` / `pub fn`, `let` / `let mut`, `if` / `else if`, `while`, `match` on Option/Result, `import`, `return`.

## String walk (unicode scalar)

| API | Notes |
|---|---|
| `.len()` | **byte** length (Rust `String::len`) |
| `chars_len(s)` | unicode scalar count |
| `char_at(s, i)` | i-th scalar as 1-char String |
| `str_slice(s, start, end)` | char indices `[start, end)` |
| `char_is_digit` / `char_is_alpha` / `char_is_space` | single-char String |

## I/O (sealed, real)

| API | Requires | Behavior |
|---|---|---|
| `read_file(path)` / `.read_file` | `&FsCap` on caller | `Result[String, String]` disk read |
| `write_file(path, content)` / `.write_file` | `&FsCap` | `Result[Void, String]` disk write |
| `env_get(key)` | `&EnvCap` | `Result[String, String]` |

## Process surface

- `ooda run file.oo -- arg1 arg2 …` injects argv into `main(args: List[String], …)` (also `argv`).
- Capability params (`fs: &FsCap`, …) still injected as opaque tokens.

## oodac representation convention

```
type Token = struct {
    kind: Int,
    line: Int,
    col: Int,
    text: String
};
// AST: arena of records + Int node kinds + child indices (not recursive user enums)
```

## Explicitly out of freeze

`for` sugar, user enums, net/async/crypto/json product surface, LSP/pkg, full SPEC, WASM as bootstrap gate.

## Host-only kill dates

| Feature | Host | LLVM native | Kill / target |
|---|---|---|---|
| List / String walk / struct | M0 | deferred | M4 progressive EMIT |
| FsCap I/O / argv | M0 | N/A (runtime) | runtime support or host lib at M4 |

## Examples

- `examples/chs_fs_roundtrip.oo`
- `examples/chs_list_string.oo`
- `examples/chs_struct_token.oo`
- `examples/chs_token_walk.oo` — integration golden
