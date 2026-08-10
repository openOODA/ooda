# GitHub Actions: product workflow (already in tree)

**Workflow file:** [`.github/workflows/product.yml`](../.github/workflows/product.yml)  
**Display name:** `product`  
**Job:** `product rails`

This is the remote product rail. It does **not** install rustup, cargo, or rustc.
It shadows `cargo`/`rustc` on `PATH`, resolves a pure `SEED_OODAC`, then runs
`scripts/ci_product.sh` and an offline `scripts/install_dress_rehearsal.sh`.

**Not a beta gate.** Green here proves the pure product path on a clean
runner; it does not authorize a beta tag (`bootstrap/BETA.md`).

## Seed strategy (first hit wins)

1. `bootstrap/seed/oodac` if executable in the checkout (see `bootstrap/seed/README.md`)  
2. `SEED_OODAC` env (workflow_dispatch / future secrets — prefer public pin assets)  
3. GitHub Release tarball for `install/BOOTSTRAP_PIN`:
   - `ooda-<pin>-linux-x86_64.tar.gz`
   - matching `.sha256` sidecar (required; fail-closed if missing)
   - optional `workflow_dispatch` input `seed_url` (allowlisted to this repo’s Releases only)

## How to verify green

### On GitHub (UI)

1. Open https://github.com/openOODA/ooda/actions/workflows/product.yml  
2. Latest run on `main` (or your PR) should show **success** for job `product rails`.  
3. Open the run log: seed step should print `seed: local …`, `seed: SEED_OODAC env`, or `seed: release <pin>`; rails end with `ci_product: PASSED`.

### CLI (`gh`)

Workflow **file** is `product.yml`; display **name** is `product`. Prefer file or display name accordingly:

```bash
# Last 3 runs of this workflow
gh run list -R openOODA/ooda -w product --limit 3
# equivalent:
gh run list -R openOODA/ooda --workflow=product.yml --limit 3

# Inspect a run
gh run view <run-id> -R openOODA/ooda
gh run view <run-id> -R openOODA/ooda --log-failed
```

Expect `conclusion: success` when the pin asset (or in-tree seed) is coherent.

### Local proxy (same rails, no GHA)

```bash
export SEED_OODAC="${SEED_OODAC:-$PWD/oodac/oodac}"
# or: cp -a oodac/oodac bootstrap/seed/oodac && chmod +x bootstrap/seed/oodac
./scripts/ci_product.sh
./scripts/seed_dress_rehearsal.sh          # seed path only
./scripts/install_dress_rehearsal.sh       # release layout
```

## Preflight for green remote CI

| Check | Why |
|-------|-----|
| `install/BOOTSTRAP_PIN` matches a published tag | Download URL uses that pin |
| Release has `ooda-<pin>-linux-x86_64.tar.gz` **and** `.sha256` | CI fails closed without sidecar |
| Seed binary is pure product `oodac` (not Cargo host) | Bootstrap path only |
| No secrets in workflow or seed directory | See `bootstrap/seed/README.md` |

```bash
PIN=$(tr -d '\r\n' < install/BOOTSTRAP_PIN)
gh release view "$PIN" -R openOODA/ooda
# Assets must include the tarball and .sha256 for linux-x86_64
```

If the release is empty, either publish via `scripts/release.sh` + upload, or place
`bootstrap/seed/oodac` for a fork/private runner (gitignored — not the default
public CI path).

## Recorded `gh run list` (Loop4 snapshot)

Command (2026-08-06 UTC):

```text
$ gh run list -R openOODA/ooda -w product --limit 3
# (formerly workflow display name: no-rust / file: no_rust.yml)
```

| Status | Title (abbrev) | Branch | Event | Run ID | Created (UTC) | URL |
|--------|----------------|--------|-------|--------|---------------|-----|
| **failure** | test(fixtures): fix FS roundtrip… | main | push | 31121080779 | 2026-08-06T16:48:00Z | https://github.com/openOODA/ooda/actions/runs/31121080779 |
| **failure** | fix(oodac): for/match fail-closed… | main | push | 31121022298 | 2026-08-06T16:46:57Z | https://github.com/openOODA/ooda/actions/runs/31121022298 |
| **failure** | fix(patch): confine under cwd… | main | push | 31120820113 | 2026-08-06T16:43:21Z | https://github.com/openOODA/ooda/actions/runs/31120820113 |

### Failure residual (honest)

Latest failed step: **Resolve SEED_OODAC (pin / local / release)** — `curl` exit 22 / HTTP **404** on

`https://github.com/openOODA/ooda/releases/download/v0.184.0-alpha/ooda-v0.184.0-alpha-linux-x86_64.tar.gz`

Facts checked 2026-08-06:

| Fact | Result |
|------|--------|
| Tag / release `v0.184.0-alpha` | Present |
| API assets | `ooda-v0.184.0-alpha-linux-x86_64.tar.gz` + `.sha256` (`state: uploaded`) |
| Unauthenticated browser download URL | **404** (repo is **private**) |
| Checkout seed `bootstrap/seed/oodac` | Absent (gitignored; not in CI tree) |

Workflow file and local rails are present. Remote green residual: either (1) make
release assets downloadable to the runner (token-authenticated curl / `gh release
download`, or public release assets), or (2) supply `bootstrap/seed/oodac` on the
job without committing secrets. Purely documenting here — do not force a beta tag.

Re-check after residual fix:

```bash
gh run list -R openOODA/ooda -w product --limit 3
# want: conclusion success on latest main push / workflow_dispatch
```
## Related

- `bootstrap/seed/README.md` — place seed, sha256, never commit secrets  
- `scripts/seed_dress_rehearsal.sh` — offline seed → `bootstrap_no_cargo`  
- `scripts/install_dress_rehearsal.sh` — offline release layout  
- `scripts/ci_product.sh` — local product rail  
- `bootstrap/RELEASE_CHECKLIST.md` — pin + asset + sha256 coherence  
- monorepo `PROGRESS.md` / latest release notes — gate honesty (no beta force)
