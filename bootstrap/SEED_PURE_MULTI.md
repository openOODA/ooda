# M15 — Cold seed pure multi host

## What the rail proves

`bootstrap/seed/oodac` can act as the **emit host** for pure multi-module rebuild of
tree `oodac/main.oo` **without** requiring tree-host `oodac/oodac` on that rail.

Proof path (side binary only — never clobber the working product host):

```bash
source ~/.local/ooda-toolenv/env.sh 2>/dev/null || true
export PURE_NO_ARC=0 PURE_SKIP_CHECK=1
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
OODAC_BIN=./bootstrap/seed/oodac \
  bash scripts/oodac_pure_build.sh oodac/main.oo "$TMPDIR/oodac_from_seed"
"$TMPDIR/oodac_from_seed" check bootstrap/corpus/check/pass/ok_main.oo
```

Mechanical smoke: `scripts/seed_pure_multi_smoke.sh` (wired in `scripts/ci_product.sh`).

## Current status (probe)

**GREEN** (2026-08-09, M15 closed).

Cold seed as emit host for pure multi of tree `oodac/main.oo` succeeds.
Mechanical: `scripts/seed_pure_multi_smoke.sh` → `PASSED (green)`.

| Observation | Result |
|-------------|--------|
| `OODAC_BIN=bootstrap/seed/oodac` + pure multi of `oodac/main.oo` | `OK_PURE_MULTI` |
| Module graph | `unique_fns=352 from_modules=131` |
| Side binary `check` on `ok_main.oo` | OK |
| Product pin | free-safe seed/product binary preferred for `--json-errors` path |

No **active** residual line is present (smoke greps only for a start-of-line
`ACTIVE: RESIDUAL_SEED_PURE_MULTI…` status line — prose/examples do not count).

**Honesty:** if tip sources later break under seed emit, flip this section to residual
and add the active marker line (below). Never claim seed is current when pure multi fails.
Match-assign free-ARC json UAF (outer mut freed at if scope_exit) is fixed in tip
`c_emit_match.oo` (`decl=0` → `c_env_put_last`); rebuild free self-host before
claiming seed/product free-safe for `--json-errors`. Prefer a known free-safe
seed pin when refreshing until stage-2 re-prove.

## Residual honesty (if seed lags later)

If cold seed cannot emit current tree modules (mis-type / `ERR_EMIT` / bad binary):

1. Prefer green: rebuild with a trusted tree host, then refresh seed (below).
2. Do **not** claim seed is current.
3. Document lag with an **active** status line the smoke greps (`^ACTIVE: RESIDUAL_SEED_PURE_MULTI`):


   (That ACTIVE line must be left-flush at column 0 — not in a fenced block.)

When that active line is present **and** pure multi fails, `seed_pure_multi_smoke.sh`
prints `RESIDUAL_SEED_PURE_MULTI` and exits 0 (CI honesty is mechanical).
When the active line is absent and pure multi fails, the smoke **fails closed**.

Other known non-blocking notes (not M15 residual):

- Prefer tree `oodac/oodac` for day-to-day pure multi after free-ARC when available;
  cold seed is the **bootstrap / offline** emit host.
- Seed binary is gitignored; pin/release assets cover remote CI when local seed is absent.
- Refreshing seed from an untrusted or half-built host is out of scope without owner approval.

## How to refresh seed

Only after a **trusted** pure rebuild (e.g. green `bootstrap_no_cargo` / fixed-point):

```bash
# Working product host is the trusted pure rebuild output:
cp -a oodac/oodac bootstrap/seed/oodac
chmod +x bootstrap/seed/oodac
sha256sum bootstrap/seed/oodac > bootstrap/seed/oodac.sha256
```

Or extract from a release pack (see `bootstrap/seed/README.md`).

**Never** copy a Cargo/host-built binary into `bootstrap/seed/`.
**Never** claim seed is current if pure multi of tip sources fails under it.

## Related

| Path | Role |
|------|------|
| `scripts/oodac_pure_build.sh` | pure multi path (emit each `.oo`, link once) |
| `scripts/bootstrap_no_cargo.sh` | product rebuild; seed is emit host |
| `scripts/fixed_point.sh` | seed preference includes `bootstrap/seed/oodac` |
| `scripts/seed_dress_rehearsal.sh` | offline bootstrap dress (full product path) |
| `scripts/seed_pure_multi_smoke.sh` | **this** M15 side-path pure multi rail |
| `scripts/pure_build_fp_smoke.sh` | M20 input_fp stability (content only; not bit-identical bins) |
| `bootstrap/PURE_BUILD_FP.md` | pure multi input fingerprint contract |
| `bootstrap/seed/README.md` | seed placement / integrity |
| `bootstrap/ARC_M2_RESIDUAL.md` | ARC residual (orthogonal to seed multi green) |
