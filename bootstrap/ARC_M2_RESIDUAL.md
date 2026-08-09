# M2 ARC residual (honest)

**Status:** reclaim **in progress** — formal strip + alias retain land; runtime free still **leak-safe**. Not beta.

## Shipped (re-runnable)

| Piece | Behavior |
|-------|----------|
| `PURE_NO_ARC=0` default | Pure rebuild keeps retain/release in C |
| Runtime release | `oo_str_release` / list release **decrement only** (no `free` when ref hits 0) |
| Seed softener | `scripts/pure_rewrite_formals.py` via `oodac_pure_rewrite.py` — strips formal-param releases (seed treated formals as owned) |
| Alias retain (tree emit) | `let x = y` for T/S/I emits retain of `x` when `y` is env-tagged ARC |
| `arc_smoke.sh` | early_return + concat reassign execute under leak-safe ARC |
| ARC-on self-host | seed emit + formal strip + leak-safe free → working `oodac` / `bin/ooda` bootstrap |

## Why free is not claimed

Re-enabling `free` on ref_count==0 crashed seed-emitted pure multi of oodac (tcache UAF / bad emit). Formal strip alone was not enough; alias/get retain path still incomplete for full self-host reclaim.

## Still residual

1. **Real free** when ref_count hits 0 — blocked until seed emit ownership is correct end-to-end (or tree emit is the only pure multi host).
2. **Seed still emit host** for pure multi of oodac (tree emit intermittent / incomplete).
3. **Borrow from `slist_get`** may still need retain in more call sites (tree emit).
4. **Broader arc suite** (lists, nested scopes) not fully inventory-proved.
5. **Not beta.**

## Rebuild

```bash
export PURE_NO_ARC=0 PURE_SKIP_CHECK=1
OODAC_BIN=./bootstrap/seed/oodac bash scripts/oodac_pure_build.sh oodac/main.oo oodac/oodac
./scripts/bootstrap_no_cargo.sh
bash scripts/arc_smoke.sh
```

## Related

- `runtime/chs_rt_str.c`, `chs_rt_list.c` (leak-safe residual comments)
- `scripts/pure_rewrite_formals.py`, `scripts/oodac_pure_rewrite.py`
- `oodac/c_emit_let.oo`, `oodac/c_emit_arc.oo`
