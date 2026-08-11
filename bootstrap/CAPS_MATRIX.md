# Capability matrix (claimed pure path)

**Purpose:** Map each sealed effect op through **check → emit-c → runtime → product**.  
**Rules:** Default-deny. Unfinished = fail-closed, never silent ambient I/O. `fetch` is product-lowered; other net names residual.  
**Runtime seal:** sealed FS/Sys/Env/Net/Time/Rand/Alloc ops re-check process-local capability tokens at native runtime. Canonical: [`STATIC_CAPS.md`](STATIC_CAPS.md).  
**Cap vs FFI (PM 6.3 path A):** bare `dlopen` / host-FFI free names need `&UnsafeFFICap` at check. Process-local Fs/Sys/… do **not** seal full C TCB / OS `dlopen` / raw pointers / `import "C"` — see [`CAP_FFI.md`](CAP_FFI.md).  

**Sources:** `oodac/check_caps.oo`, `oodac/c_emit_lower.oo`, `runtime/chs_rt_fs.c`, `runtime/chs_rt_sys.c`, `chs_rt_time_rand.c`, `chs_rt_alloc.c`.

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
| `read_file` | `&FsCap` | sealed free call; deny without param | `oo_read_file(cap, path)` | `chs_rt_fs.c` + `oo_cap_require` | **real** (static+runtime) |
| `write_file` | `&FsCap` | sealed free call | `oo_write_file(cap, path, content)` | `oo_write_file` + require | **real** (static+runtime) |
| `path_exists` | `&FsCap` | sealed free call | `oo_path_exists(cap, path)` | require + fopen probe | **real** |
| `file_size` | `&FsCap` | sealed free call | `oo_file_size(cap, path)` | require + ftell | **real** |
| `sys_exec` | `&SysCap` | sealed free call + arg-flow | multi-arg → `oo_sys_exec`; single → `oo_sys_exec1` | `fork`+`execvp` (AUDIT R2/R3 closed; not `system(3)`) | **real** |
| `sys_spawn` / `sys_wait` / `sys_kill` | `&SysCap` | sealed free call + arg-flow | `oo_sys_spawn` / `oo_sys_wait` / `oo_sys_kill` | require Sys then **Err residual** (path A seal; no real fork/wait/kill) | **fail-closed residual** — use `sys_exec` for blocking spawn+wait; `std/os/process.oo` wrappers |
| `env_get` | `&EnvCap` | sealed free call | `oo_env_get(cap, key)` | require + getenv | **real** |
| `fetch` | `&NetCap` | sealed free call; allow only with `NetCap` | `oo_fetch(cap, url)` | `chs_rt_sys.c` + net cap require; HTTP/1.0 GET | **real** (AUDIT R9) |
| `http_get` / `net_get` / `net_connect` / `downloadData` / `query_remote_api` | `&NetCap` | sealed free call | **explicit `ERR\tc_emit\tnet residual`** | none | **fail-closed residual** |
| `now_ms` | `&TimeCap` | sealed free call | `oo_now_ms(cap)` | `chs_rt_sys.c` + time require; `CLOCK_REALTIME` ms | **real** (M12) |
| `sleep_ms` | `&TimeCap` | sealed free call | `oo_sleep_ms(cap, ms)` | time require + `nanosleep` | **real** (M12) |
| `random` | `&RandCap` | sealed free call | `oo_random(cap)` | rand require + host entropy / LCG fallback (not crypto claim) | **real** (M12) |
| `seed` | `&RandCap` | sealed free call | `oo_seed(cap, s)` | rand require; seeds process LCG | **real** (M12) |
| `alloc_bytes` | `&AllocCap` | sealed free call | `oo_alloc_bytes(cap, n)` | alloc require; returns n as smoke size token (not real heap sandbox) | **real** (M17) |
| `free_bytes` | `&AllocCap` | sealed free call | `oo_free_bytes(cap, p)` | alloc require; no-op free by handle | **real** (M17) |
| `process_exit` | none (ambient) | not sealed | `oo_process_exit` | `exit` | **real** (not a cap class) |
| `list_new` / `list_push` / string concat | **none (ambient residual)** | not sealed | ambient CHS | no AllocCap gate | **intentional alpha residual** — sealing would brick pure compiler |

Aliases sealed but not product-lowered: `fs_read`/`fs_write` (Fs), `exec`/`spawn_process`/`async_spawn_internal` (Sys), `env_set`/`getenv` (Env) — check deny without cap; emit leaves name as-is → link fail if used (fail-closed residual).

---

## Layer notes

### Check
- Free-call scan inside each `fn` body: `IDENT` + `LPAREN` matched against `sealed_kind_of` / `is_sealed_{net,fs,sys,env,time,rand,alloc}`.
- Cap **param** grant only for type after `COLON` + `AMP` + Cap IDENT (e.g. `fs: &FsCap`, `time: &TimeCap`, `alloc: &AllocCap`).
- Cap **arg-flow (F01):** free call first arg (or method receiver `fs.read_file`) must be an IDENT naming a param of that cap class — not merely “param present somewhere.”
- Fixtures: `check/fail/cap_arg_not_passed.oo`, `cap_arg_wrong_name.oo`; pass method: `check/pass/ok_method_fs_read.oo`.
- Residual: dynamic/computed callees not scanned; cap only as param name (not expression). Ambient `list_new` not sealed.

### Emit (Backend-C)
- Cap tokens compile to `long long`; `main` injects `oo_cap_grant_fs/sys/env/net/time/rand/alloc/ffi/thread/gpu()` (process-local; grant idents match param names).
- Sealed FS/Env/Sys/Net/Time/Rand/Alloc/FFI/Thread/Gpu lowers **pass the leading cap arg** (ABI with runtime).
- Sealed **libfloor residual** (`mutex_*`/`thread_spawn`/`gpu_launch`/`tcp_*`/…): after require, return `Result` **Err** path A (not real OS threads/GPU/sockets).
- Non-IDENT first arg on sealed ops: `ERR\tc_emit\t… requires &…Cap` (fail-closed).

### Runtime: process-local token seal
- **Canonical:** [`STATIC_CAPS.md`](STATIC_CAPS.md).
- Each lowered sealed op calls `oo_cap_require_*` before ambient libc/clock/entropy/explicit alloc helper.
- Forged / zero / classic `0x4F4F*` token → non-zero exit + `ERR\tcap\t…` (not ambient effect).
- Not unforgeable object-caps across hostile binary rewrite; not crypto CSPRNG / attested time / OS rlimit heap isolation.
- Net: only `fetch` product-lowered; do not add ambient curl/socket without a real design.
- Alloc: smoke returns size token / no-op free — **not** a claim of heap sandboxing.

---

## Fixtures (immune system)

| Class | Pass (has cap) | Fail (no cap) |
|-------|----------------|---------------|
| Net | `check/pass/ok_net_cap_fetch.oo` | `check/fail/no_cap_fetch.oo` |
| Fs read | `check/pass/ok_fs_read.oo` | `check/fail/no_cap_read_file.oo` |
| Fs write | `check/pass/ok_fs_write.oo` | `check/fail/no_cap_write_file.oo` |
| Fs path | `check/pass/ok_path_exists.oo` | `check/fail/no_cap_path_exists.oo` |
| Sys | `check/pass/ok_sys_exec.oo` | `check/fail/no_cap_sys_exec.oo` |
| Sys spawn (path A residual) | `check/pass/ok_sys_spawn.oo` | `check/fail/no_cap_sys_spawn.oo` |
| Env | `check/pass/ok_env_get.oo` | `check/fail/no_cap_env_get.oo` |
| Time | `check/pass/ok_now_ms.oo` | `check/fail/no_cap_now_ms.oo` |
| Rand | `check/pass/ok_random.oo` | `check/fail/no_cap_random.oo` |
| Alloc | `check/pass/ok_alloc_bytes.oo` | `check/fail/no_cap_alloc_bytes.oo` |
| Pure no-effect | `check/pass/ok_main.oo` | — |
| Runtime seal | `emit-c/pass/cap_runtime_read.oo` + forge deny in smoke | — |

Runtime round-trip: `fixtures/chs_fs_roundtrip.oo` (Fs). Smoke: `scripts/caps_matrix_smoke.sh` (Fs/Sys/Env/Net/Time/Rand) + `scripts/alloc_cap_smoke.sh` (Alloc + forge-cap deny).

---

## Expanding the sealed table

1. Add name to `is_sealed_*` / `sealed_kind_of` in `check_cap_util.oo`.
2. Add **pass + fail** check fixtures.
3. Either lower in `c_emit_lower.oo` + runtime symbol with `oo_cap_require`, **or** explicit emit residual (like net).
4. Never widen allow without re-running deny fixtures. Do **not** seal ambient `list_new` without a full compiler redesign.

*P1 BUILD_OUT: Caps completeness on claimed path.*
