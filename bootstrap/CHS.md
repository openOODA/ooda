# CHS_lang / OODA₀ — Compiler Host Subset (**FROZEN at M1**)

**Status:** Frozen for bootstrap (M1+). Stage-0 host + C backend native emit.  
**Constitution:** `DESIGN.md` unchanged.  
**Self-host:** CHS frontend fixed-point green via `scripts/fixed_point.sh` (native `oodac` + token digests + smoke C).

## Dual-engine / multi-backend done-definition

| Surface | Interpreter (`ooda run`) | Native |
|---|---|---|
| CHS host surface (M0) | Required | C backend (`ooda build --target c`) + `runtime/chs_rt.c` + gcc |
| LLVM integer subset | N/A | clang link when available (optional) |
| Full SPEC product | Not required for CHS | Not required |

A feature is **CHS-complete for bootstrap** when:

1. Interpreter tests pass, and  
2. C backend can emit+link programs that use it (or it is I/O/runtime-only with sealed caps), and  
3. Unfinished product work still fails non-zero.

## Types (in)

| Type | Notes |
|---|---|
| `Int`, `Bool`, `String`, `Void` | |
| `Option[T]`, `Result[T, E]` | host; C lowers Result as `OoResS`/`OoResV` |
| `List[Int]`, `List[String]` | `list_new` / `list_push` / `list_get` / `list_len` / `.len` |
| `type T = struct { fields }` | field access `t.f`; arena+int tags for AST |
| `&FsCap`, `&EnvCap`, … | opaque tokens; real FS/env builtins |

## Control (in)

`fn` / `pub fn`, `let` / `let mut`, `if` / `else if`, `while`, `match` Option/Result, `import`, `return` (including nested in `if`).

## String walk (in)

| API | Notes |
|---|---|
| `.len()` | **byte** length |
| `chars_len` / `char_at` / `str_slice` | unicode scalar indices |
| `char_is_digit` / `char_is_alpha` / `char_is_space` | single-char String |

## I/O (in, sealed)

`read_file` / `write_file` under `&FsCap`; `env_get` under `&EnvCap`.

## Process (in)

`ooda run f.oo -- args…` → `main(args: List[String], …)`.

## Explicitly OUT of freeze

`for` sugar, user enums, traits, net/async/crypto/json product, LSP/pkg, full WASM product, PyTorch.

## Canonical dumps (stage-0)

```
ooda dump tokens <file>   # KIND\tLINE\tCOL\tTEXT
ooda dump ast <file>      # structural line dump
ooda dump check <file>    # OK / ERR\t…
```

## oodac

- Source: `oodac/main.oo` (CHS only)
- Native: `ooda build --target c oodac/main.oo` → links `libooda.a` + `runtime/chs_rt.c`
- Commands:
  - `tokens <file>` — **lexer implemented in `.oo`** (parity vs `ooda dump tokens`)
  - `ast <file>` — **exact** stage-0 AST via `host_ast_dump` (same code as `ooda dump ast`)
  - `check <file>` — **exact** stage-0 type+cap via `host_check` (same as `ooda dump check`)
  - `build <src> [out]` — **real** CHS native compile via `chs_build` → CCodeGen+gcc (not hardcoded C)
- Parity: `scripts/chs_parity.sh` (pass + fail lex corpus; exact AST/check)
- Fixed-point: `scripts/fixed_point.sh` (**stage-1 builds stage-2**)

## Metric (M5)

1. stage-0 builds oodac → **stage-1**  
2. stage-1 **`build`s** CHS smoke (`chs_list_string.oo`) to a real executable and runs it  
3. stage-1 **`build`s oodac/main.oo → stage-2** (not stage-0 twice)  
4. token digests s0 ≡ s1 ≡ s2  
5. stage-2 builds smoke successfully  
6. intentional digest drift fails  

Not used: raw binary hash alone; hardcoded `puts("chs-smoke-ok")` theater.
