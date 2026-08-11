# Stable diagnostic codes (agent routing)

**Purpose:** Machine-readable codes for `ooda check --json-errors` / `oodac check --json-errors`.  
Agents should branch on **`code`**, not free-text `msg`.

**Shape (JSON array; one object per diagnostic):**

```json
[
  {
    "code": "E_CAP",
    "line": 2,
    "col": 13,
    "msg": "…",
    "path": "fixtures/example.oo",
    "fix_hint": "Add matching &FsCap/…/&AllocCap param; sealed ops need a capability."
  }
]
```

- **Pass (no diagnostics):** `[]` and exit 0.  
- **Fail:** non-empty array and exit non-zero.  
- **`fix_hint`:** code-keyed **narrative** agent guidance from `diag_fix_hint(code)` only (not AST rewrite / auto-apply; not host AiDiagnostic). Depth covers E_CAP (Fs/Sys/Env/Net/Time/Rand/**Alloc**), E_TC, E_PARSE (brace/token), E_CHECK (structural), E_LEX, E_LOAD, E_EMIT, E_CLI, E_BUILD, E_BACKEND.  
- **Security:** payloads are diagnostics only. `path` / `msg` are JSON-escaped strings (not filesystem open APIs). Do not treat `msg` as a path to open.

## Code table

| Code | ERR kind (human) | Meaning | Hint focus |
|------|------------------|---------|------------|
| `E_CAP` | `capability` | Capability seal violation (missing NetCap/FsCap/SysCap/Env/Time/Rand/**Alloc**) | add matching `&*Cap` param |
| `E_TC` | `type` | Typecheck / name / refinement failure | define symbol / arity / annotations |
| `E_PARSE` | `parse` | Parse unexpected token / structure | balance braces/parens; fix token near loc |
| `E_LEX` | `lex` | Lexer failure | remove/escape bad character |
| `E_CHECK` | `check` | Check-stage structural (empty, no_fn, …) | non-empty source + at least one fn |
| `E_LOAD` | `load` | Source load / missing file | path exists and readable |
| `E_BUILD` | `build` | Build / link failure | inspect compile/link output |
| `E_BACKEND` | `backend` | Unsupported `--backend` | product floor is `--backend c` |
| `E_EMIT` | `c_emit` | Backend-C emit failure | simplify residual / emit-c corpus |
| `E_CLI` | `cli` | Product CLI residual / usage | `ooda help` + flags/paths |
| `E_SECRET` | `secret` | Secret→println bare IDENT refuse (M52–M60 path A/B) | remove sink / do not print SECRET name |
| `E_MAX_CYCLES` | `max_cycles` | Cycle fuel exceeded or invalid N (M48/M54/M58) | lower loops or raise MAX_CYCLES; not OS cgroup |
| `E_CONTRACT` | `contract` | Simple requires/ensures violation or residual complex | fix simple shape or residual pack |
| `E_OTHER` | *(unknown)* | Unclassified line in capture | see this doc |

## CLI

```text
oodac check <file.oo> --json-errors
oodac check <file.oo> -json
ooda  check <file.oo> --json-errors
```

Human mode (default) still prints `ERR\t<kind>\t<message>` tab lines.

## Rails

- `scripts/json_errors_smoke.sh` — pass (`[]`) + fail shape checks:
  - **E_CAP** — non-empty `fix_hint` with capability guidance (AllocCap path included in table)
  - **E_TC** — non-empty `fix_hint` (undefined var)
  - **E_PARSE** — non-empty `fix_hint` with parse/brace/token guidance (`corpus/parse/fail/missing_brace.oo`)
- Corpus: `bootstrap/corpus/check/pass|fail`, `bootstrap/corpus/typecheck/fail/undefined_var.oo`, `bootstrap/corpus/parse/fail/`
| E_RESIDUAL | residual | Residual DESIGN free-call path A default-deny (M153) |

| E_HITL | hitl | Non-interactive // HITL: pause deny-mode (M157) |
