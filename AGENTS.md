# openOODA — agent token discipline

Agents blow budgets by pasting pure-C dumps, full SPRINT, and multi-hundred-line
gcc logs. **Orient with compression tools first.** Full source is a last resort.

## Hard rules (token traps)

| Never auto-read | Why |
|-----------------|-----|
| `~/.cache/ooda-tmp/**/all*.c` | ~700KB pure emit |
| Full `openOODA/SPRINT.md` | multi-k lines; use residual slice |
| Full `emit-c` stdout of oodac modules | use emit_health / outline |
| 500+ line gcc logs | use `ooda_err_digest.sh` |
| Entire `oodac/*.oo` tree in one prompt | 152 modules; outline per file |

## Prefer (cheap orient)

```bash
# session start (~1–3k tokens)
./scripts/ooda_agent_digest.sh --emit-sample

# one module surface (~1% of source for outline)
./oodac/oodac outline oodac/c_emit_let_ext.oo
./oodac/oodac reflect oodac/c_emit_let_ext.oo

# symbol + outline JSON
./scripts/ooda_context_pack.sh oodac/check_caps.oo check_function
./scripts/ooda_product_context.sh oodac/check_caps.oo check_function

# emit matrix (status only, no C)
./scripts/ooda_emit_health.sh check_caps check_drive c_emit_fn main

# compress a log
./scripts/ooda_err_digest.sh /tmp/pure_build.err
```

## Work loop (OODA, token-aware)

1. **Observe:** `ooda_agent_digest.sh` + emit_health on suspects — not full tree.
2. **Orient:** `outline` / `reflect` / `context_pack` for the 1–3 files in the critical path.
3. **Decide:** one residual claim; write a **minimal repro** (.oo ≤30 lines) before editing.
4. **Act:** surgical edit; smoke with **exit code + last line only**.
5. **Leave-off:** ≤15 lines (tip hash, green list, residual, next). No all.c.

## When full source is justified

- File ≤80 lines, or
- You already have a SEGV/hang line number and need ±20 lines, or
- `--force-source` on `ooda_context_pack.sh` after outline proved insufficient.

## Product floors (don’t re-litigate every session)

- Tip host: pure multi seed+ABI (`oodac.m17x`) — product AGY/m169 green.
- M171: untyped `field_at` let SEGV fixed in `c_emit_let_ext.oo`.
- Residual: pure self-host **emit quality** (double `fs`, mangled idents), not SEGV.

## Complement, don’t race

Library surface / std growth may be owned by other agents. This tree’s
high-leverage agent work is **compiler self-host integrity + token tooling**.
