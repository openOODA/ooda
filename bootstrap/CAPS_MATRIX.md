# Capability matrix (claimed pure path)

**Catalog (token list):** process doc [`../../openOODA/CAPS.oot`](../../openOODA/CAPS.oot) — 27 V2 names; **type accept ≠ CAP-G1 FIXED**.  
**Net (CAP-G1):** **OPEN residual** — close FAIL 2026-08-12 (`../../openOODA/audit/cap_g1_phalanx_rollup.oot`). Tip check still seals net ops as **`NetCap`** via `sealed_kind_of`; preferred granular map / wrong-granular deny is WIP (live host observed HttpCap+`tcp_bind` accept). Dirty runtime may wire per-op `oo_cap_require_{http,tcp,udp,bind}` — not a close prove. Non-net op→granular still residual (SPRINT **CAP-G2+**).  
**Purpose:** Map each sealed effect op through **check → emit-c → runtime → product**.  
**Rules:** Default-deny. Unfinished = fail-closed, never silent ambient I/O. `fetch` is product-lowered; other http-ish names residual emit.  
**Runtime seal:** sealed FS/Sys/Env/Net/Time/Rand/Alloc ops re-check process-local capability tokens at native runtime. Canonical: [`STATIC_CAPS.md`](STATIC_CAPS.md). **Not** full OS isolation / complete `seccomp-bpf` product claim.  
**Cap vs FFI (PM 6.3 path A):** bare `dlopen` / host-FFI free names need `&UnsafeFFICap` at check. Process-local Fs/Sys/… do **not** seal full C TCB / OS `dlopen` / raw pointers / `import "C"` — see [`CAP_FFI.md`](CAP_FFI.md).  

**Sources:** `oodac/check_caps.oo`, `oodac/check_cap_util.oo`, `oodac/c_emit_lower.oo`, `runtime/chs_rt_fs.c`, `runtime/chs_rt_sys.c`, `runtime/chs_rt_netfloor.c`, `chs_rt_time_rand.c`, `chs_rt_alloc.c`.

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
| `write_file` | `&FsCap` | sealed free call | `oo_write_file(cap, path, content)` | require + **`OODA_FS_WRITEDIR` fail-closed** (path realpath under dir; empty/unset → deny) | **real** path A — residual: **parent-realpath for new files** (nonexistent leaf `realpath` fails) |
| `path_exists` | `&FsCap` | sealed free call | `oo_path_exists(cap, path)` | require + fopen probe | **real** |
| `file_size` | `&FsCap` | sealed free call | `oo_file_size(cap, path)` | require + ftell | **real** |
| `sys_exec` | `&SysCap` | sealed free call + arg-flow | multi-arg → `oo_sys_exec`; single → `oo_sys_exec1` | `fork`+`execvp` (not `system(3)`); child env **filtered** to `OODA_`/`OO_` + minimal `PATH=/usr/bin:/bin` | **real** path A (T3) |
| `sys_spawn` / `sys_wait` / `sys_kill` | `&SysCap` | sealed free call + arg-flow | `oo_sys_spawn` / `oo_sys_wait` / `oo_sys_kill` | require Sys then **Err residual** (path A seal; no real fork/wait/kill) | **fail-closed residual** — use `sys_exec` for blocking spawn+wait; `std/os/process.oo` wrappers |
| `sys_epoll_create` / `sys_inotify_init` / `sys_prctl` | `&SysCap` | sealed free call + arg-flow | `oo_sys_epoll_create` / `oo_sys_inotify_init` / `oo_sys_prctl` | require Sys then **Err residual** (M166 path A; not full async I/O) | **fail-closed residual** — `std/os/process.oo` thin wrappers |
| `env_get` | `&EnvCap` | sealed free call | `oo_env_get(cap, key)` | require + `oo_process_policy_getenv` (**only** `OODA_`/`OO_` keys) | **real** path A |
| `fetch` | **preferred `&HttpCap`** (legacy **`&NetCap` supersede**) | sealed free call; exact preferred **or** `NetCap`; wrong granular refuse | `oo_fetch(cap, url)` | `chs_rt_sys.c` + `oo_cap_require_http` (http **or** net token); HTTP/1.0 GET | **real** path-A (R9 + CAP-G1) |
| `tcp_connect` | **preferred `&TcpCap`** (`&NetCap` supersede) | sealed free call + arg-flow | `oo_tcp_connect` | `chs_rt_netfloor.c` + `oo_cap_require_tcp`; real sockets; **keep fd open**; Ok(`"fd:N"`) | **real** (M162/M166 + CAP-G1) |
| `tcp_bind` | **preferred `&BindCap`** (`&NetCap` supersede) | sealed free call + arg-flow | `oo_tcp_bind` | `oo_cap_require_bind`; listen loopback; keep-open Ok(`"fd:N"`) | **real** (M162/M166 + CAP-G1) |
| `bind_udp` | **preferred `&UdpCap`** (`&NetCap` supersede) | sealed free call + arg-flow | `oo_bind_udp` | `oo_cap_require_udp`; keep-open Ok(`"fd:N"`) | **real** (M162/M166 + CAP-G1) |
| `tcp_write` / `tcp_read` / `tcp_close` | **preferred `&TcpCap`** (`&NetCap` supersede) | sealed free call + arg-flow | `oo_tcp_write` / `oo_tcp_read` / `oo_tcp_close` | `oo_cap_require_tcp`; fd-slot String IO; close frees slot | **real** (M166 + CAP-G1) — not full HTTP/3/gRPC |
| `udp_recv` | **preferred `&UdpCap`** (`&NetCap` supersede) | sealed free call + arg-flow | `oo_udp_recv` | `oo_cap_require_udp`; fd-slot recv as String | **real** (M166 + CAP-G1) |
| `sock_raw` / `raw_socket` | **`&NetCap` only** (no soft granular) | sealed free call | `oo_sock_raw` | `oo_cap_require_net`; always **Err** residual (`SOCK_RAW not product`) | **fail-closed residual** (M166) |
| `tls_connect` | **preferred `&TcpCap`** (`&NetCap` supersede) | sealed free call + arg-flow | `oo_tls_connect` | `chs_rt_tls.c` + `oo_cap_require_tcp`; residual **or** OpenSSL when `OO_HAVE_OPENSSL=1` | **residual default** / **real TLS when OpenSSL linked** (M163). Optional `OODA_TLS_INSECURE_TCP=1` → Ok TCP-only (**insecure residual**, not TLS) |
| `http_get` / `net_get` / `net_connect` / `downloadData` / `query_remote_api` | **preferred `&HttpCap`** (`&NetCap` supersede) | sealed free call (check same as fetch) | **explicit `ERR\tc_emit\tnet residual`** | none | **fail-closed residual** (check prefers HttpCap; no product lower) |
| `now_ms` | `&TimeCap` | sealed free call | `oo_now_ms(cap)` | `chs_rt_sys.c` + time require; `CLOCK_REALTIME` ms | **real** (M12) |
| `sleep_ms` | `&TimeCap` | sealed free call | `oo_sleep_ms(cap, ms)` | time require + `nanosleep` | **real** (M12) |
| `random` | `&RandCap` | sealed free call | `oo_random(cap)` | rand require + host entropy / LCG fallback (not crypto claim) | **real** (M12) |
| `seed` | `&RandCap` | sealed free call | `oo_seed(cap, s)` | rand require; seeds process LCG | **real** (M12) |
| `alloc_bytes` / **`malloc`** | `&AllocCap` | sealed free call | `oo_alloc_bytes(cap, n)` | alloc require; returns n as smoke size token (not real heap sandbox) | **real** (M17; M166 alias) |
| `free_bytes` / **`free`** | `&AllocCap` | sealed free call | `oo_free_bytes(cap, p)` | alloc require; no-op free by handle | **real** (M17; M166 alias) |
| **`realloc`** | `&AllocCap` | sealed free call | free then `oo_alloc_bytes` | path A quota-token adjust (not OS realloc) | **path A** (M166) |
| `mutex_lock` / `mutex_unlock` | `&ThreadCap` | sealed free call | `oo_mutex_lock` / `oo_mutex_unlock` | `chs_rt_libfloor.c` pthread mutex slots | **real** (M162) |
| `thread_spawn` | `&ThreadCap` | sealed free call | `oo_thread_spawn` | `chs_rt_thread.c` joinable pthread; Ok(`"tid:N"`) | **real** (M163 join path A) |
| `thread_join` | `&ThreadCap` | sealed free call | `oo_thread_join` (Int) / `oo_thread_join_s` (String `"tid:N"`) | `pthread_join` slot table | **real** (M163) |
| `channel_new` / `channel_send` / `channel_recv` | `&ThreadCap` | sealed free call | `oo_channel_new` / `oo_channel_send` / `oo_channel_recv` | `chs_rt_channel.c` process-local bounded string queue (16×8) | **real** (M164 path A) |
| `gpu_launch` | `&GpuCap` | sealed free call | `oo_gpu_launch` | noop/cpu: Ok; else **Err** no device shaders | **Path A (M165)** — no CUDA device |
| `process_exit` | none (ambient) | not sealed | `oo_process_exit` | `exit` | **real** (not a cap class) |
| `list_new` / `list_push` / string concat | **none (ambient residual)** | not sealed | ambient CHS | no AllocCap gate | **intentional alpha residual** — sealing would brick pure compiler |

Aliases sealed but not product-lowered: `fs_read`/`fs_write` (Fs), `exec`/`spawn_process`/`async_spawn_internal` (Sys), `env_set`/`getenv` (Env) — check deny without cap; emit leaves name as-is → link fail if used (fail-closed residual).

---

## Layer notes

### Check
- Free-call scan inside each `fn` body: `IDENT` + `LPAREN` matched against `sealed_kind_of` / `is_sealed_{net,fs,sys,env,time,rand,alloc}`.
- **CAP-G1 net:** `sealed_kind_of` → `sealed_net_kind_of` preferred (`HttpCap`/`TcpCap`/`UdpCap`/`BindCap`/`NetCap`). Grant bag: exact preferred **or** legacy `NetCap` supersede for soft-granular. Arg-flow: `preferred:id` **or** `NetCap:id`. Wrong granular (e.g. only `HttpCap` on `tcp_bind`) → deny. Helpers: `cap_net_granted_ok` / `cap_net_argflow_ok` in `check_cap_util.oo`.
- Cap **param** grant only for type after `COLON` + `AMP` + Cap IDENT (e.g. `http: &HttpCap`, `net: &NetCap`, `fs: &FsCap`, `time: &TimeCap`).
- Cap **arg-flow (F01):** free call first arg (or method receiver `fs.read_file`) must be an IDENT naming a param of that cap class — not merely “param present somewhere.”
- Fixtures: `check/fail/cap_arg_not_passed.oo`, `cap_arg_wrong_name.oo`; pass method: `check/pass/ok_method_fs_read.oo`; net legacy pass: `check/pass/ok_net_cap_fetch.oo` (still `&NetCap` supersede).
- Residual: dynamic/computed callees not scanned; cap only as param name (not expression). Ambient `list_new` not sealed. CAP-G1 preferred pass fixtures exist (`ok_http_cap_fetch` / `ok_tcp_cap_connect` / `ok_net_cap_fetch_tcp`); dedicated wrong-granular fail fixtures still residual.

### Emit (Backend-C)
- Cap tokens compile to `long long`; `main` injects `oo_cap_grant_fs/sys/env/net/time/rand/alloc/ffi/thread/gpu()` plus **partial** granular (`tcp` / `fsread` / `process` today) — full V2 grant matrix residual (**CAP-G3**).
- Sealed FS/Env/Sys/Net/Time/Rand/Alloc/FFI/Thread/Gpu lowers **pass the leading cap arg** (ABI with runtime).
- Sealed **ThreadCap product (M162/M163/M164):** `mutex_lock`/`mutex_unlock` real pthread mutex; `thread_spawn` joinable pthread → Ok(`"tid:N"`); `thread_join(slot)` joins slot (or `oo_thread_join_s` parses `"tid:N"`); `channel_new`/`channel_send`/`channel_recv` process-local bounded string queues. Not detach-by-default. **GpuCap** `gpu_launch` (M165): noop/`cpu:` Ok honesty; device shaders **Err residual**. Actor model residual.
- Net libfloor (CAP-G1): `tcp_bind` → require_bind; `tcp_connect`/`tcp_*` IO → require_tcp; `bind_udp`/`udp_recv` → require_udp; `fetch` → require_http; each accepts matching granular **or** NetCap token. `sock_raw` NetCap-only residual Err; `tls_connect` require_tcp (OpenSSL residual — M163).
- Non-IDENT first arg on sealed ops: `ERR\tc_emit\t… requires &…Cap` (fail-closed).

### Runtime: process-local token seal
- **Canonical:** [`STATIC_CAPS.md`](STATIC_CAPS.md).
- Each lowered sealed op calls `oo_cap_require_*` before ambient libc/clock/entropy/explicit alloc helper.
- Forged / zero / classic `0x4F4F*` token → non-zero exit + `ERR\tcap\t…` (not ambient effect).
- **Not** unforgeable object-caps across hostile binary rewrite; **not** crypto CSPRNG / attested time / OS rlimit heap isolation; **not** full OS isolation / complete `seccomp-bpf`/`SIGSYS` product floor (CAP-G5 residual honesty).
- **T3 path A (runtime ZT):** `sys_exec` child env = `OODA_`/`OO_` only + minimal PATH; FFI handle table mutex + same-thread nested `oo_dlopen` hard refuse ([`CAP_FFI.md`](CAP_FFI.md)); `write_file` fail-closed under `OODA_FS_WRITEDIR`.
- **T4 hygiene residuals (not closed):** bak/side binary ignore policy; `tools/minisign` not vendored; `chs_rt_ffi.c` monofile >350; 8.1 DEBUG reclassified as product ERR diagnostics — see [`AUDIT_RESIDUAL.md`](AUDIT_RESIDUAL.md) §T4.
- **T3 residuals (explicit):** full IFC; unrestricted any-path `dlopen`; refcount UAF beyond ARC path-A free-on-ref0; parent-realpath for **new** write paths; full C TCB seal.
- Net (CAP-G1 path-A): `fetch` + TCP/UDP product-lowered with **per-op** require_http/tcp/udp/bind (NetCap supersede); **fd slots keep sockets open** after connect/bind (M166). Byte IO is `tcp_read`/`tcp_write`/`udp_recv` as String (not true `&[u8]`). **TLS residual** unless `OO_HAVE_OPENSSL=1`. **No** full HTTP/3/gRPC/SOCK_RAW product. Smokes: `scripts/libfloor_net_smoke.sh`, `scripts/tcp_io_smoke.sh`, `scripts/tls_path_a_smoke.sh`.
- Alloc: smoke returns size token / no-op free — **not** a claim of heap sandboxing.

---

## Fixtures (immune system)

| Class | Pass (has cap) | Fail (no cap) |
|-------|----------------|---------------|
| Net (legacy supersede) | `check/pass/ok_net_cap_fetch.oo` (`&NetCap` on `fetch`) | `check/fail/no_cap_fetch.oo` |
| Net CAP-G1 preferred | `check/pass/ok_http_cap_fetch.oo` (`&HttpCap`); `ok_tcp_cap_connect.oo` (`&TcpCap`); `ok_net_cap_fetch_tcp.oo` (NetCap supersede multi-op) | `no_cap_fetch.oo` (missing); dedicated wrong-granular fail fixtures residual (check rules refuse wrong preferred) |
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

**M166 std cap scoping samples (path A):** effectful `std/os/*` take leading `&SysCap`/`&NetCap`/`&ThreadCap`/`&FsCap`; pedagogical fixtures `sys_syscall_path_a.oo` / `libfloor_tcp_io.oo` + smokes `sys_syscall_path_a_smoke.sh` / `tcp_io_smoke.sh` prove main-with-cap, sealed first-arg, bare refuse. **CAP-G1:** prefer `&HttpCap`/`&TcpCap`/`&UdpCap`/`&BindCap` on new net APIs; `&NetCap` remains valid supersede. Std majority still legacy NetCap (CAP-G6 migration). **Residual open:** cap forgery via `as fn(...)` cast (AGY) — not full forgery fix; full OS isolation not claimed. Canonical writeup: [`STATIC_CAPS.md`](STATIC_CAPS.md) § M166 path A + [`CAPS.oot`](../../openOODA/CAPS.oot) CAP-G1 map.

---

## Expanding the sealed table

1. Add name to `is_sealed_*` / `sealed_kind_of` in `check_cap_util.oo`.
2. Add **pass + fail** check fixtures.
3. Either lower in `c_emit_lower.oo` + runtime symbol with `oo_cap_require`, **or** explicit emit residual (like net).
4. Never widen allow without re-running deny fixtures. Do **not** seal ambient `list_new` without a full compiler redesign.

*P1 BUILD_OUT: Caps completeness on claimed path.*
