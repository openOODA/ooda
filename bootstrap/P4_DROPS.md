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
| Product LLVM **floor** (link/run/optimize) | **No** — Backend-C remains product self-host floor |
| `oodac emit-llvm` / `ooda build --target llvm` / `--emit-llvm` | **Emit smoke** (textual IR written; `llvm_token_align_smoke`) — **not** clang/llc product rails |
| `--release` optimize path | **Fail-closed** residual on pure CLI |

**Honest product claim:** openOODA **self-hosts on Backend-C**. LLVM IR **emit**
exists as a partial M5 surface; do **not** market full LLVM backend, optimize,
or production link. Optional host clang link of **C** output is incidental.

**Close full LLVM product only with:** FLOOR F3-style MVP + rails + DESIGN-aligned notes.

See: `cli/main.oo`, `scripts/ooda_product.sh`, `bootstrap/FLOOR.md`, `BACKEND_F3_PREP.md`.

---

## WASM product path

| Claim | Status |
|-------|--------|
| Product WASM **run** (wasmtime/WASI host rails) | **No** |
| `ooda build --target wasm` / `oodac emit-wasm` | **Emit smoke** (`.wat` text; `wasm_emit_smoke`) — not product execute path |
| Full second-backend self-host on WASM | **No** |

**Honest product claim:** WASM **text emit** is partial M4; not a shipped WASM
runtime product. Fixture `*.wat` leftovers are demos, not a floor.

**Close full WASM product only with:** F3 candidate **W** + runtime pin + run rails.

Rails today: `scripts/wasm_emit_smoke.sh` (emit only).

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
| `--json-errors` on **run** | Fail-closed residual (check path is real) |
| `--fuzz` | **Un-gated** → Python harness residual (`FUZZ_DEFER.md`); not pure-native |
| `oodac --backend llvm\|wasm` | Accepted for emit scaffolding; product **self-host** remains Backend-C |
| Host preamble decls (`oo_host_*`) | Backend-C link residual, not a second backend |

---

*Update when an item ships an MVP or when owner permanently rejects it. Do not
delete a row to look cleaner — mark **shipped** or **rejected** with a pin.*
