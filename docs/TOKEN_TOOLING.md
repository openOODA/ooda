# Token-minimized APIs — product surface (path A)

## Why

Agent sessions die from pure `all.c` (~700KB), full SPRINT, emit dumps, gcc walls.
SPEC §12 requires outline/reflect/context. **Product CLI is the capability.**

## Product commands (`./bin/ooda`)

| Command | Tokens out | Use |
|---------|------------|-----|
| `ooda outline <file>` | ~1% source | public signatures |
| `ooda reflect <file> [sym]` | tiny JSON | caps + sig |
| `ooda context <file> [sym]` | slice JSON | est_tokens_full vs slice |
| `ooda pack <file> [sym]` | human pack | outline+reflect+slice |
| `ooda digest` | ~3k chars | session orient |
| `ooda health [mods]` | ~20B/mod | emit OK/SEGV (no C) |
| `ooda err-digest [log]` | ~0.5–1k | gcc/ERR compression |

Also: `oodac outline|reflect|context` on the compiler binary.

Backend scripts live under `scripts/ooda_*.sh` and are **dispatched only** via
`scripts/ooda_product.sh` (invoked by pure `cli/main.oo`). Agents should call
`ooda …`, not invent parallel tooling.

## Measured

`c_emit_ty.oo` (11665 bytes) → outline ~154 bytes (~1.3%).

## Residual

- `--json-minimal` diagnostics (SPEC) not fully product
- Package-distributed outlines (RP-2.2)
- Full dual-green rebuild of tip after every CLI edit (tip host is m172+)

## See also

`AGENTS.md` — hard token traps for any agent on this tree.
