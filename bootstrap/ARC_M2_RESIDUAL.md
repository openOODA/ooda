# M2 ARC residual (honest)

**Status:** self-host no longer **requires** `PURE_NO_ARC=1` strip; **not beta**.  
**Default:** `PURE_NO_ARC=0` in `bootstrap_no_cargo.sh` (retain/release kept in pure-built C).

## What is green (re-runnable)

| Piece | Evidence |
|-------|----------|
| Pure rebuild without strip | `PURE_NO_ARC=0` pure_build of `oodac` + `cli` produces working binaries |
| Bootstrap smoke | version / check / build+exec CHS fixture |
| `arc_smoke.sh` | `early_return_string` + `string_concat_reassign` emit+gcc+run |
| Emit ARC present | Generated C still contains `oo_str_retain` / `oo_str_release` |
| Env put in-place | Scope-safe rebind; stage-1 CLI emit under strip path still OK |
| println bare String call | `__fr__` → `oo_print_str` on tree/stage-1 |

## Honest residual (not closed)

1. **Runtime release does not free** — `oo_str_release` / list release decrement refcounts but **do not `free`**, because seed-era (and some tree) emit still over-releases / UAF. Memory **leaks** instead of heap-corrupting. True reclaiming ARC is future work.
2. **Tree emit host intermittent SEGV** under `EMIT_NO_CONCAT` on some modules — pure_build retries 3× then falls back to `bootstrap/seed/oodac`.
3. **Cold seed still used** as emit host for reliable pure multi-module builds.
4. **Not object-caps / not beta.**

## Rebuild

```bash
export PURE_NO_ARC="${PURE_NO_ARC:-0}"
export PURE_SKIP_CHECK=1
OODAC_BIN=./bootstrap/seed/oodac bash scripts/oodac_pure_build.sh oodac/main.oo oodac/oodac
./scripts/bootstrap_no_cargo.sh
bash scripts/arc_smoke.sh
```

## Related

- `runtime/chs_rt_str.c` / `chs_rt_list.c` — leak-safe release
- `scripts/oodac_pure_build.sh` — emit retry + seed fallback
- `scripts/arc_smoke.sh`
