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
    "fix_hint": "Add matching &FsCap/… capability param."
  }
]
```

- **Pass (no diagnostics):** `[]` and exit 0.  
- **Fail:** non-empty array and exit non-zero.  
- **`fix_hint`:** code-keyed agent guidance only (not AST rewrite; not host AiDiagnostic).  
- **Security:** payloads are diagnostics only. `path` / `msg` are JSON-escaped strings (not filesystem open APIs). Do not treat `msg` as a path to open.

## Code table

| Code | ERR kind (human) | Meaning |
|------|------------------|---------|
| `E_CAP` | `capability` | Capability seal violation (missing NetCap/FsCap/SysCap) |
| `E_TC` | `type` | Typecheck / name / refinement failure |
| `E_PARSE` | `parse` | Parse unexpected token / structure |
| `E_LEX` | `lex` | Lexer failure |
| `E_CHECK` | `check` | Check-stage structural (empty, no_fn, …) |
| `E_LOAD` | `load` | Source load / missing file |
| `E_BUILD` | `build` | Build / link failure |
| `E_BACKEND` | `backend` | Unsupported `--backend` |
| `E_EMIT` | `c_emit` | Backend-C emit failure |
| `E_CLI` | `cli` | Product CLI residual / usage |
| `E_OTHER` | *(unknown)* | Unclassified line in capture |

## CLI

```text
oodac check <file.oo> --json-errors
oodac check <file.oo> -json
ooda  check <file.oo> --json-errors
```

Human mode (default) still prints `ERR\t<kind>\t<message>` tab lines.

## Rails

- `scripts/json_errors_smoke.sh` — pass (`[]`) + fail (cap, undefined var) shape checks  
- Corpus: `bootstrap/corpus/check/pass|fail`, `bootstrap/corpus/typecheck/fail/undefined_var.oo`
