# `ooda outline` / `ooda reflect` (agent tooling)

**Status:** product CLI (alpha). **Security:** parse-only — never executes user `.oo`.  
**Implementation:** pure `oodac outline` / `oodac reflect` via product CLI (M1).  
**Residual:** not a full typed AST / import graph; not a substitute for `check`.

## Commands

```text
ooda outline <file.oo>
ooda reflect <file.oo> [symbol]
```

| Command | Purpose | Exit |
|---------|---------|------|
| `outline` | Token-cheap **public** API list | `0` ok; `2` missing/unreadable file; `1` empty / no `pub fn` / parse fail |
| `reflect` | Richer symbol/contract/cap/verify metadata (NDJSON) | same; `1` if optional `symbol` not found |

Missing path or unreadable file → fail-closed (`ERR	outline|reflect	unreadable file: …`).

---

## Outline format (one line per `pub fn`)

```text
pub fn NAME(param: Type, …) [-> Ret] [caps=Cap1,Cap2]
```

- Only **`pub fn`** symbols (private `fn` omitted).
- Parameter types as written (including `&FsCap` etc.).
- Return type omitted when absent / void-like.
- `caps=` only when a param type matches `NetCap|FsCap|SysCap|EnvCap` (with optional `&`).
- Bodies, `requires`/`ensures`, and `verify` blocks are **not** printed (token-cheap).

**Example** (`fixtures/int_main.oo`):

```text
pub fn add(a: Int, b: Int) -> Int
pub fn main()
```

**Example with caps** (`fixtures/chs_fs_roundtrip.oo`):

```text
pub fn main(fs: &FsCap) caps=FsCap
```

---

## Reflect format (JSON lines / NDJSON)

One JSON object per line, **source order**. Compact separators (no pretty-print).

### Function line

```json
{"kind":"fn","name":"add","pub":true,"params":[{"name":"a","type":"Int"},{"name":"b","type":"Int"}],"ret":"Int","requires":["a >= 0","b >= 0"],"ensures":["result >= 0"],"caps":[]}
```

| Field | Meaning |
|-------|---------|
| `kind` | `"fn"` |
| `name` | function name |
| `pub` | `true` if `pub fn` |
| `params` | `[{name,type},…]` types as source text |
| `ret` | return type string (empty if none) |
| `requires` | source-like precondition texts (may be empty) |
| `ensures` | source-like postcondition texts |
| `caps` | ordered unique `FsCap`/`NetCap`/`SysCap`/`EnvCap` from params |

### Verify line

```json
{"kind":"verify","name":"add"}
```

Optional second arg filters to items whose `name` matches (function and/or verify of that name). Unknown symbol → non-zero + `ERR	reflect	symbol not found: …`.

**Example** (`fixtures/hello.oo`):

```text
{"kind":"fn","name":"greet",…,"requires":[],"ensures":[],"caps":[]}
{"kind":"verify","name":"greet"}
{"kind":"fn","name":"main",…}
```

(Complex contracts like `name.len() > 0` are residual on Backend-C emit — fail-closed; outline/reflect still may surface them when present.)

---

## Security / honesty

1. **No execution** of the input program (no `oodac build` / `run` / `check` on the target for these commands).
2. Text scan only: top-level `fn` / `pub fn` / `verify` + brace balance; not a full SPEC parser.
3. Residual vs DESIGN: no `--json` outline variant yet; no type-alias / import lines in outline MVP.

## Rails

- `scripts/outline_reflect_smoke.sh` — pass + fail (unreadable, missing symbol).
- Product CLI rebuild: `oodac build cli/main.oo bin/ooda` (or `bootstrap_no_cargo.sh`).
