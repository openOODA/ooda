# M2 ARC residual (honest)

**Status:** partial progress in tree; **not** M2-complete; **not** beta.  
**Default self-host path still requires `PURE_NO_ARC=1`.** Do not claim ARC-on stage2 safe.

Seed-era emit inserts retain/release that heap-corrupts rebuilt `oodac`/`bin/ooda` when linked with current `chs_rt` unless those calls are stripped. That strip remains the supported pure rebuild path.

---

## What landed (tree WIP — keep residual honest)

| Piece | Behavior |
|-------|----------|
| **Env last-wins** | `c_emit_env.oo`: `c_env_get` / put last-wins (kind map for emit; S/I/T and meta keys) |
| **Local-only releases** | `c_emit_local_releases`: body locals only (skip pre-scope params); safe for early return |
| **`c_emit_ret_arc` stash-before-release** | Non-void expr always stashed into `__ret_val` **before** local releases (no UAF on expr return) |
| **Named local return** | Retain-before-release still used when returning a named local of kind T/S/I |
| **Emit-order probe** | `fixtures/arc_smoke/early_return_string.oo` emit order **PASS** after pure rebuild under residual path |

Related sources: `oodac/c_emit_env.oo`, `oodac/c_emit_arc.oo`, `oodac/c_emit_stmt.oo`.

---

## Still deferred (do not market closed)

1. **`PURE_NO_ARC=0` stage2 self-host** — ARC-on pure rebuild of compiler/CLI as seed successor. Historically heap/UAF risk; **not proven green**.
2. **Full ARC proof suite run-green** — not only emit-order; execute smokes under ARC-on emit (`fixtures/arc_smoke/*`, string concat reassign, early return, lists).
3. **println of bare String-returning calls** — `println(make(1))` still may lower to `oo_print_int`; bind via `let s: String = make(1)` (as in updated early_return fixture) for `oo_print_str`.
4. **Default flip** — `bootstrap_no_cargo.sh` / pure_build must keep `PURE_NO_ARC=1` until success criteria below hold.

`RELEASE_NOTES_v0.183.0-alpha.md` residual line predates stash-before-release wiring; this file is the live honesty note for M2 ARC.

---

## How to rebuild (supported residual path)

```bash
# From ooda/ — seed host emit, strip seed ARC, pure multi-module link
export SEED_OODAC="${SEED_OODAC:-./bootstrap/seed/oodac}"
export PURE_NO_ARC="${PURE_NO_ARC:-1}"
export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-1}"
# Prefer seed as emit host when rebuilding compiler (tree oodac may still be hostile on some paths):
OODAC_BIN=seed PURE_NO_ARC=1 ./scripts/oodac_pure_build.sh
# Full product path:
./scripts/bootstrap_no_cargo.sh
```

Smokes known green under residual (`PURE_NO_ARC=1`) path include: `outline_reflect_smoke`, `llvm_token_align_smoke`, `wasm_emit_smoke`, `bc_vm_smoke`, honesty probes, `chs_list_string` via ooda build; line lock OK. Those do **not** prove ARC-on self-host.

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
