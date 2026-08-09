# M2 ARC residual (honest)

**Status:** partial progress in tree; **not** M2-complete; **not** beta.  
**Default self-host path still requires `PURE_NO_ARC=1`.** Do not claim ARC-on stage2 safe.

Seed-era emit inserts retain/release that heap-corrupts rebuilt `oodac`/`bin/ooda` when linked with current `chs_rt` unless those calls are stripped. That strip remains the supported pure rebuild path.

---

## What landed (tree WIP — keep residual honest)

| Piece | Behavior |
|-------|----------|
| **Env last-wins** | `c_emit_env.oo`: `c_env_get` last-wins; `c_env_put` in-place replace (keeps scope frame; reassign-in-if no longer migrates outer locals) |
| **Local-only releases** | `c_emit_local_releases`: body locals only (skip pre-scope params); safe for early return |
| **`c_emit_ret_arc` stash-before-release** | Non-void expr always stashed into `__ret_val` **before** local releases (no UAF on expr return) |
| **Named local return** | Retain-before-release still used when returning a named local of kind T/S/I |
| **arc_smoke (2 fixtures)** | `scripts/arc_smoke.sh` **execute PASS** for `early_return_string` + `string_concat_reassign` under tree/stage-1 host (emitted C still has retain/release). Cold seed host still fails those fixtures (gcc type/proto errors). |
| **println bare String-return** | `c_emit.oo` / `c_emit_fn.oo` program-level `__fr__name=T` + `c_expr_is_str_env` → `println(make(1))` lowers to `oo_print_str` on tree/stage-1. Seed host still lowers to `oo_print_int`. |
| **Stage-1 emit host for CLI** | With `c_env_put` in-place: stage-1 under `PURE_NO_ARC=1` can `emit-c cli/main.oo` and `oodac_pure_build.sh cli/main.oo` (outline/reflect `list_push(fwd, sym)` stays `oo_slist_push`). |
| **Untyped str-concat let** | `c_emit_let.oo`: early-out `oo_str_lit`/`oo_str_concat` → `OoStr` (avoids PURE_NO_ARC SEGV in `c_rhs_call_name`/mega-table). `cli/product_sh.oo` also annotates `let sh: String`. |

Related sources: `oodac/c_emit_env.oo`, `oodac/c_emit_arc.oo`, `oodac/c_emit_stmt.oo`, `oodac/c_emit_let.oo`, `oodac/c_emit.oo`, `oodac/c_emit_fn.oo`, `oodac/c_emit_ops.oo`.

---

## Still deferred (do not market closed)

1. **`PURE_NO_ARC=0` stage2 self-host** — ARC-on pure rebuild of compiler/CLI as seed successor. Historically heap/UAF risk; **not proven green**.
2. **Full ARC proof suite run-green** — two fixtures execute green under tree/stage-1 (see landed). **Not closed:** lists coverage, broader suite, cold-seed emit host parity, and ARC-on pure rebuild of oodac/CLI (item 1). Fixture ARC emit ≠ self-host ARC-on.
3. **println of bare String-returning calls** — **fixed** on tree/stage-1 via `__fr__` fn_env (see landed). Seed still wrong (`oo_print_int`); not a seed product claim.
4. **Default flip** — `bootstrap_no_cargo.sh` / pure_build must keep `PURE_NO_ARC=1` until success criteria below hold.
5. **Stage-1 as emit host for full CLI** — **fixed** (env put in-place): reassign of `List[String]` inside `if` no longer migrates kind into the if-frame / loses it on scope_exit, so outline/reflect `list_push(fwd, sym)` stays `oo_slist_push`. Stage-1 `PURE_NO_ARC=1` can `emit-c cli/main.oo` and pure_build CLI. Bootstrap may still prefer cold seed as emit host for other residual reasons; ARC-on stage2 remains deferred (items 1–2, 4).

`RELEASE_NOTES_v0.183.0-alpha.md` residual line predates stash-before-release wiring; this file is the live honesty note for M2 ARC.

---

## How to rebuild (supported residual path)

```bash
# From ooda/ — seed host emit, strip seed ARC, pure multi-module link
export PURE_NO_ARC="${PURE_NO_ARC:-1}"
export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-1}"
# Prefer cold seed as emit host (not a half-built tree oodac/oodac):
OODAC_BIN=./bootstrap/seed/oodac PURE_NO_ARC=1 \
  bash scripts/oodac_pure_build.sh oodac/main.oo oodac/oodac
# Full product path (bootstrap prefers bootstrap/seed/oodac when SEED_OODAC unset):
./scripts/bootstrap_no_cargo.sh
```

Smokes known green under residual (`PURE_NO_ARC=1`) path include: `outline_reflect_smoke`, `llvm_token_align_smoke`, `wasm_emit_smoke`, `bc_vm_smoke`, honesty probes, `chs_list_string` via ooda build, `arc_smoke` (2 fixtures, tree/stage-1 host); line lock OK. Those do **not** prove ARC-on self-host (`PURE_NO_ARC=0`).

---

## Success criteria (before flipping `PURE_NO_ARC` default off)

All must hold with evidence (rails + re-runnable notes). Meeting them = eligible to change the default; **not** auto-claim M2/beta.

1. **Stage2 ARC-on:** pure rebuild with `PURE_NO_ARC=0` produces working `oodac` and `bin/ooda` (no heap abort/SEGV on bootstrap smoke).
2. **Fixed-point:** stage-N vs N+1 digests match under ARC-on pure path (or documented equivalent of `fixed_point.sh`).
3. **ARC suite run-green:** `fixtures/arc_smoke/*` (at least early_return_string + string_concat_reassign) **execute** green under ARC-on emit, not emit-only.
4. **No silent strip:** product pure_build no longer depends on stripping retain/release for self-host correctness.
5. **Honesty update:** this file + release residual language updated; `PURE_NO_ARC` default may then move to `0` in `bootstrap_no_cargo.sh` / rewrite path.

Until then: **keep `PURE_NO_ARC=1` default.** Prefer residual over soft-pass.

## Related

- `scripts/bootstrap_no_cargo.sh` — default `PURE_NO_ARC=1`
- `scripts/oodac_pure_build.sh` / `oodac_pure_rewrite.py` — strip path
- `bootstrap/FUZZ_DEFER.md` — M2 ARC interaction under fuzz residual
- `bootstrap/AUDIT_RESIDUAL.md` — R6 string free / arena residual
- `fixtures/arc_smoke/` — ARC probes
)
