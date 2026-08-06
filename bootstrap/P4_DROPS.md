# P4 permanent drops / fail-closed residuals (honesty)

**Purpose:** Record stretch items that are **not product claims** until a deliberate
MVP lands. Prefer this document over silent stubs or half-implemented flags.

**Product rule:** unfinished = **fail-closed** (non-zero exit + `ERR… residual`),
not soft-pass, not “OK_HOST”.

**Beta:** drops here do **not** block alpha pins. Owner may keep them out of
`BETA.md` Part B forever.

---

## LLVM / production optimize path

| Claim | Status |
|-------|--------|
| Product LLVM backend | **No** |
| `--emit-llvm`, `--release` optimize path | **Fail-closed** on pure CLI |
| `ooda build --target llvm` | **Fail-closed** (`ERR cli … residual/beta-out`) |

**Permanent product claim until reversed:** openOODA ships **Backend-C only**
(FLOOR). LLVM is **not** a supported floor. Optional host clang link of C output
is incidental, not an LLVM product backend.

**Re-open only with:** FLOOR F3-style MVP + rails + DESIGN-aligned notes — not a
drive-by flag.

See: `cli/main.oo`, `bootstrap/FLOOR.md`, `bootstrap/BACKEND_F3_PREP.md`.

---

## WASM product path

| Claim | Status |
|-------|--------|
| Product WASM emit/run | **No** |
| `ooda build --target wasm` | **Fail-closed** |
| oodac `wasm` command | **Fail-closed** (beta-out surface) |

**Permanent product claim until reversed:** WASM is **out of product**. Demos or
fixture `*.wat` leftovers are not a product path.

**Re-open only with:** second-backend MVP (F3 candidate **W**) + runtime pin +
smoke rails. Until then residual is intentional.

Rails: `scripts/p3_no_cargo_smoke.sh`, `scripts/beta_cli_smoke.sh`.

---

## Packaging / registry

| Claim | Status |
|-------|--------|
| `ooda pkg` / package registry | **No product registry** |
| Cap-honest package install from index | **Not shipped** |

**Permanent residual:** no product package manager or registry client in the pure
CLI. Install is **release tarball + pin** (`install/`, `scripts/release.sh`),
not a crates.io-like index.

If packaging returns, it must stay **fail-closed without net/cap**, verify
checksums/signatures, and never soft-skip verify (historical host pkg lessons).

---

## Concurrency / async

| Claim | Status |
|-------|--------|
| Async runtime / green threads in product | **No** |
| Concurrent I/O product surface | **No** |

**Not in product.** Any future concurrency must be **DESIGN-aligned** (caps,
fail-closed effects, no silent data races) and scheduled as explicit build-out —
not a drive-by language feature.

Until then: sequential CHS/Backend-C semantics only.

---

## Related residual (not P4-only, still honest)

| Item | Residual |
|------|----------|
| Cold-start seed | Prebuilt pure `oodac` once (`SEED_OODAC` / release asset) |
| `--json-errors`, `--fuzz` | Fail-closed on pure path |
| Non-`c` `--backend` | Fail-closed (`FLOOR.md`) |
| Host preamble decls (`oo_host_*`) | Backend-C link residual, not a second backend |

---

*Update when an item ships an MVP or when owner permanently rejects it. Do not
delete a row to look cleaner — mark **shipped** or **rejected** with a pin.*
