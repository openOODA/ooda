# Token-minimized agent tooling (path A)

## Why

Agent sessions on openOODA die from context bloat: pure `all.c` (~700KB), full
SPRINT, emit-c dumps, and gcc walls. SPEC already claims outline/reflect/context
for 80–90% savings; these scripts make that operational **now**.

## Tools

| Script | Tokens out | Use |
|--------|------------|-----|
| `scripts/ooda_agent_digest.sh` | ~1–3k | Session start orient |
| `scripts/ooda_emit_health.sh` | ~20B/module | SEGV/TO/OK matrix |
| `scripts/ooda_err_digest.sh` | ~0.5–1k | Compress gcc/ERR logs |
| `scripts/ooda_context_pack.sh` | ~0.2–2k | outline+reflect+symbol slice |
| `scripts/ooda_product_context.sh` | JSON ~0.5k | CLI context backend |

Built-ins (prefer over `cat`):

```text
oodac outline <file>   # signatures only (often ~1% of source)
oodac reflect <file>   # JSON caps + sigs
```

## Measured

`oodac/c_emit_ty.oo` (11665 bytes) → outline ~154 bytes (~1.3%).

## Residual

- `ooda context file#sym` product CLI wiring if not yet dispatched to these scripts
- `--json-minimal` diagnostics (SPEC) not fully product
- Package-distributed outlines (RP-2.2 future)

## Agent rule file

See `AGENTS.md` in this package for hard token traps.
