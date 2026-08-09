# M2 ARC residual (honest)

**Status:** free reclaim **blocked for product self-host**. Softeners + nested-block ARC land; runtime free **leak-safe**. Not beta.

## Shipped (re-runnable)

| Piece | Behavior |
|-------|----------|
| `PURE_NO_ARC=0` default | Pure rebuild keeps retain/release in C |
| Runtime release | **decrement only** (no `free` on ref 0) for str + lists |
| Formal softener | `pure_rewrite_formals.py` — formal-param releases + formal reassign `__tmp` |
| Alias softener | `pure_rewrite_alias_retain.py` — retain after bare `OoT x = y` |
| `oo_slist_get` | retains returned string |
| Tree alias retain | `c_emit_let_alias_retain` for `let x = y` |
| Nested bare blocks | `c_emit_stmt` → `c_emit_block` + C `{`/`}` + scope releases |
| `arc_smoke.sh` | 4 fixtures: early_return, concat reassign, list push/get, nested_scope_str |
| ARC-on self-host | seed pure multi + leak-safe → working `oodac` / `bin/ooda` |

## Free reclaim attempts

Formals + reassign strip + alias retain still **UAF under free** for seed pure multi of oodac. Free not product-safe until tree emit owns pure multi or seed ARC is complete.

## Still residual

1. Real `free` on ref_count==0 for product self-host
2. Seed still pure-multi emit host
3. Nested bare-block **shadowing** not fully smoke-proved (C braces now emit)
4. Softeners are regex — not full ownership
5. Not beta

## Rebuild

```bash
export PURE_NO_ARC=0 PURE_SKIP_CHECK=1
OODAC_BIN=./bootstrap/seed/oodac bash scripts/oodac_pure_build.sh oodac/main.oo oodac/oodac
./scripts/bootstrap_no_cargo.sh
bash scripts/arc_smoke.sh
```

## Related

- `runtime/chs_rt_str.c`, `chs_rt_list.c`
- `scripts/pure_rewrite_formals.py`, `pure_rewrite_alias_retain.py`, `oodac_pure_rewrite.py`
- `oodac/c_emit_let.oo`, `c_emit_arc.oo`, `c_emit_stmt.oo`
