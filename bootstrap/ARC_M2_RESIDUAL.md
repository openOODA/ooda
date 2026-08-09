# M2 ARC residual (honest)

**Status:** free reclaim **blocked for product self-host**. Softeners land; runtime free **leak-safe**. Not beta.

## Shipped (re-runnable)

| Piece | Behavior |
|-------|----------|
| `PURE_NO_ARC=0` default | Pure rebuild keeps retain/release in C |
| Runtime release | **decrement only** (no `free` on ref 0) for str + lists |
| Formal softener | `pure_rewrite_formals.py` strips formal-param releases **and** formal reassign `__tmp` releases |
| Alias softener | `pure_rewrite_alias_retain.py` injects retain after `OoT x = y` bare-ident binds |
| `oo_slist_get` | retains returned string (owned let binding) |
| Tree alias retain | `c_emit_let_alias_retain` for `let x = y` |
| `arc_smoke.sh` | 3 fixtures (early_return, concat reassign, list push/get); seed ARC proto inject |
| ARC-on self-host | seed pure multi + leak-safe → working `oodac` / `bin/ooda` |

## Free reclaim attempts

| Softener | Result under free |
|----------|-------------------|
| formal strip only | tcache UAF / garbage lex on tree oodac emit |
| + formal reassign strip | still broken |
| + alias retain inject | still broken |
| str-only free | still broken on complex emit |

**Conclusion:** seed emit ownership incomplete beyond formals/aliases; free not product-safe until tree emit owns pure multi or seed ARC is fixed end-to-end.

## Still residual

1. Real `free` on ref_count==0 for product self-host
2. Seed still pure-multi emit host
3. Nested scope / more list ARC inventory
4. Not beta

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
- `oodac/c_emit_let.oo`, `oodac/c_emit_arc.oo`
