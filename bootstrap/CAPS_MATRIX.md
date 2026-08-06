# Capability matrix (claimed pure path)

**Purpose:** Map each sealed effect op through **check → emit-c → runtime → product**.  
**Rules:** Default-deny. Unfinished = fail-closed, never silent ambient I/O. Net is residual — no silent network stub.  
**Honesty residual:** caps are **static-check only** on native — no runtime re-check. Canonical: [`STATIC_CAPS.md`](STATIC_CAPS.md).  
**Sources:** `oodac/check_caps.oo`, `oodac/c_emit_lower.oo`, `runtime/chs_rt_fs.c`, preamble `oo_sys_exec1`.

Status legend:

| Status | Meaning |
|--------|---------|
| **real** | Implemented end-to-end on pure Backend-C path |
| **fail-closed residual** | Denied or hard-fail; not silently ambient |
| **check-only** | Cap gate works; no product runtime yet |

---

## Ops matrix

| Op | Cap | Check (`check_caps`) | Emit lower | Runtime | Product status |
|----|-----|----------------------|------------|---------|----------------|
| `read_file` | `&FsCap` | sealed free call; deny without param | `oo_read_file(path)` (drops cap arg) | `chs_rt_fs.c` `oo_read_file` | **real** |
| `write_file` | `&FsCap` | sealed free call | `oo_write_file(path, content)` via `c_args_drop_first` | `oo_write_file` | **real** |
| `path_exists` | `&FsCap` | sealed free call | `oo_path_exists(path)` | `oo_path_exists` | **real** |
| `file_size` | `&FsCap` | sealed free call | `oo_file_size(path)` | `oo_file_size` | **real** |
| `sys_exec` | `&SysCap` | sealed free call | `oo_sys_exec1(last_arg)` | preamble `system(3)` | **real** |
| `env_get` | `&EnvCap` | sealed free call | `oo_env_get(key)` | `chs_rt_fs.c` `oo_env_get` | **real** |
| `fetch` / `http_get` / `net_get` / `net_connect` / `downloadData` / `query_remote_api` | `&NetCap` | sealed free call; allow only with `NetCap` | **explicit `ERR\tc_emit\tnet residual`** | none | **fail-closed residual** (no silent stub) |
| `process_exit` | none (ambient) | not sealed | `oo_process_exit` | `exit` | **real** (not a cap class) |

Aliases sealed but not product-lowered: `fs_read`/`fs_write` (Fs), `exec`/`spawn_process`/`async_spawn_internal` (Sys), `env_set`/`getenv` (Env) — check deny without cap; emit leaves name as-is → link fail if used (fail-closed residual).

---

## Layer notes

### Check
- Free-call scan inside each `fn` body: `IDENT` + `LPAREN` matched against `is_sealed_{net,fs,sys,env}`.
- Cap present iff param type text is `NetCap` / `FsCap` / `SysCap` / `EnvCap` (token scan).
- **Method form:** `fs.read_file(...)` is sealed — scan is IDENT + LPAREN, so the method name is caught (not only free calls). Residual: dynamic/computed callees not scanned.

### Emit (Backend-C)
- Cap tokens compile to `int` placeholders; leading cap args dropped on sealed lowers.
- `write_file` / `read_file` / `env_get` / `path_exists` / `file_size` drop the leading cap arg when present.
- Sealed **net** ops: `process_exit(1)` with `ERR\tc_emit\tnet residual` (never emit a fake socket).

### Runtime: static-only
- **Canonical residual:** [`STATIC_CAPS.md`](STATIC_CAPS.md). Product does **not** re-check caps at native runtime.
- Check is the only seal: deny without param; net fails at emit (no silent stub).
- Backend-C lowers sealed FS/env/sys to **ambient libc** via `chs_rt` / preamble (`fopen` / `getenv` / `system`, …). Cap tokens are erased — **int placeholders**, not object-caps.
- FS + env: `runtime/chs_rt_fs.c`. Sys: inline `oo_sys_exec1` in emit preamble (`system`).
- Net: no symbols; do not add ambient curl/socket without a real NetCap product design.

---

## Fixtures (immune system)

| Class | Pass (has cap) | Fail (no cap) |
|-------|----------------|---------------|
| Net | `check/pass/ok_net_cap_fetch.oo` | `check/fail/no_cap_fetch.oo` |
| Fs read | `check/pass/ok_fs_read.oo` | `check/fail/no_cap_read_file.oo` |
| Fs write | `check/pass/ok_fs_write.oo` | `check/fail/no_cap_write_file.oo` |
| Fs path | `check/pass/ok_path_exists.oo` | `check/fail/no_cap_path_exists.oo` |
| Sys | `check/pass/ok_sys_exec.oo` | `check/fail/no_cap_sys_exec.oo` |
| Env | `check/pass/ok_env_get.oo` | `check/fail/no_cap_env_get.oo` |
| Pure no-effect | `check/pass/ok_main.oo` | — |

Runtime round-trip: `fixtures/chs_fs_roundtrip.oo` (Fs). Smoke: `scripts/caps_matrix_smoke.sh`.

---

## Expanding the sealed table

1. Add name to `is_sealed_*` in `check_caps.oo`.
2. Add **pass + fail** check fixtures.
3. Either lower in `c_emit_lower.oo` + runtime symbol, **or** explicit emit residual (like net).
4. Never widen allow without re-running deny fixtures.

*P1 BUILD_OUT: Caps completeness on claimed path.*
