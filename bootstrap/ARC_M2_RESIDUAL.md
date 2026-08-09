# M2 ARC residual (honest)

**Status:** reclaim **blocked for self-host free** — softener + get-retain land; runtime free stays **leak-safe**. Not beta.

## Shipped (re-runnable)

| Piece | Behavior |
|-------|----------|
| `PURE_NO_ARC=0` default | Pure rebuild keeps retain/release in C |
| Runtime release | **decrement only** (no `free` when ref hits 0) for str + lists |
| Seed softener | `scripts/pure_rewrite_formals.py` strips formal-param releases |
| Alias retain (tree emit) | `let x = y` for T/S/I emits retain |
| `oo_slist_get` | **retains** returned string (owned let binding; free-prep) |
| `arc_smoke.sh` | early_return + concat reassign; injects ARC protos if seed omits |
| ARC-on self-host | seed pure multi + leak-safe free → working `oodac` / `bin/ooda` |

## Free reclaim attempt (2026-08-09)

Re-enabled `free(hdr)` on ref_count==0 (str and/or list):

- **arc fixtures** historically can run under free when linked as standalone C.
- **seed pure multi of oodac** under free: `malloc(): unaligned tcache chunk` / garbage `ERR lex` when running tree oodac emit.
- Str-only free still corrupted complex emit; simple `println(1)` sometimes OK.
- ASan host lib missing on this machine (`libasan.so`); not fully stack-traced.

**Conclusion:** free is **not** product-safe until seed ownership is complete or tree emit is the only pure-multi host. Softeners (formals + get retain) stay for the next try.

## Still residual

1. **Real free** on ref_count==0 for product self-host.
2. **Seed still emit host** for pure multi of oodac.
3. **List free** not claimed even after str free works.
4. Broader arc suite (nested scopes, more lists).
5. **Not beta.**

## Rebuild

```bash
export PURE_NO_ARC=0 PURE_SKIP_CHECK=1
OODAC_BIN=./bootstrap/seed/oodac bash scripts/oodac_pure_build.sh oodac/main.oo oodac/oodac
./scripts/bootstrap_no_cargo.sh
bash scripts/arc_smoke.sh
```

## Related

- `runtime/chs_rt_str.c`, `chs_rt_list.c`
- `scripts/pure_rewrite_formals.py`, `scripts/oodac_pure_rewrite.py`
- `oodac/c_emit_let.oo`, `oodac/c_emit_arc.oo`
