# Capability seals (static + runtime)

**Status:** product floor (alpha) on pure Backend-C — PM **3.1 done (alpha)** for process-local seals (M8 + M12 Time/Rand + M17 Alloc).  
**Product rule:** claim only process-local magic-token re-check — **not** cryptographic object-caps / biometric / OS isolation.

---

## What is true today

| Layer | Behavior |
|-------|----------|
| **Check** | `oodac/check_caps.oo` — default-deny sealed free/method names; require matching `&FsCap` / `&SysCap` / `&EnvCap` / `&NetCap` / `&TimeCap` / `&RandCap` / `&AllocCap` / `&UnsafeFFICap` **param** (FFI free names: path A) |
| **Emit (Backend-C)** | Cap params → `long long`; sealed calls pass **cap IDENT first**; `main` injects **`oo_cap_grant_fs/sys/env/net/time/rand/alloc()`** (process-local tokens from entropy) |
| **Runtime (`chs_rt`)** | `oo_cap_require_*` before `read_file` / `write_file` / `path_exists` / `file_size` / `env_get` / `sys_exec` / `fetch` / `now_ms` / `sleep_ms` / `random` / `seed` / `alloc_bytes` / `free_bytes` |
| **Native binary** | Zero or classic fixed magic (`0x4F4F4653` / `…TM` / `…RN` / `…AL` etc.) as “grant” → `ERR\tcap\t…` + exit 1 |

Security for sealed effects on the claimed path:

1. **Compile-time refuse** — missing / wrong cap param → check fail  
2. **Runtime seal** — wrong token → `oo_cap_require` exit  
3. **Net** — `fetch` + TCP/UDP product-lowered; `tls_connect` residual without OpenSSL (M163; see `CAPS_MATRIX.md`)  
4. **Time / Rand** — `now_ms` / `sleep_ms` need `&TimeCap`; `random` / `seed` need `&RandCap` (process-local seal only)  
5. **Alloc** — `alloc_bytes` / `free_bytes` / M166 aliases `malloc` / `free` / `realloc` need `&AllocCap` (process-local seal only; **not** OS rlimit / heap isolation / GC)

**Honest ceiling:** process-local tokens stop accidental ambient effects (I/O, clock, entropy, explicit alloc helpers) and classic magic forges on Backend-C. They do **not** stop a hostile binary that calls `oo_cap_grant_*` or patches `oo_cap_require` out.

Classic `0x4F4F*` constants are **forged values that must be denied**, not ambient grants.  
`oo_random` uses host entropy when available (else process LCG) — **not** a cryptographic CSPRNG guarantee.

**Ambient residual (intentional):** `list_new` / `list_push` / string concat stay free for alpha — sealing them would brick the pure compiler and fixtures. Only the narrow `alloc_bytes` / `free_bytes` / `malloc` / `free` / `realloc` surface is sealed under AllocCap.

---

## What we do **not** claim

- Cryptographic / unforgeable object capabilities  
- Biometric or OS-level caps  
- Full TLS product without OpenSSL (`OO_HAVE_OPENSSL=1`); `OODA_TLS_INSECURE_TCP` is not encryption  
- Full net surface beyond `fetch` / TCP-UDP / residual-or-OpenSSL `tls_connect`  
- Cryptographically secure randomness or attested clocks  
- Heap sandboxing, ASAN, or OS `rlimit` isolation for AllocCap  
- Ambient effects without an explicit cap param on the product path  
- **Full C TCB / unrestricted OS `dlopen` / raw-pointer / Compile-Time FFI gen** — process-local Fs/Sys/… do **not** seal the whole C interop surface; path A seals named free calls under `&UnsafeFFICap` + process-local grant; M165 allowlisted OS `dlopen` (system dirs or `ALLOWDIR`) — **not** unrestricted any-path load / product `dlsym` (see [`CAP_FFI.md`](CAP_FFI.md))

---

## Rails

`scripts/caps_matrix_smoke.sh` + `scripts/alloc_cap_smoke.sh` (in `ci_product`):

- Static pass/fail corpus for FS/Sys/Env/Net/Time/Rand/Alloc  
- Product `ooda check` deny ×7 families (incl. `no_cap_now_ms`, `no_cap_random`, `no_cap_alloc_bytes`)  
- Runtime pass: Fs roundtrip, path_exists, Env, Sys argv, Time `now_ms`, Rand `random`, Alloc `alloc_bytes`  
- Runtime forge deny: Fs/Sys/Env/Net/Time/Rand/Alloc (zero + classic magic)  
- Emit fail-closed: fetch without NetCap arg; `now_ms` without TimeCap IDENT; `alloc_bytes` without AllocCap IDENT; magic-int forge build  

---

## M166 path A — std cap scoping samples

**AGY finding:** std samples “often lack” `&SysCap` / `&NetCap`. **Product floor (path A):** effectful `std/os/*` wrappers **do** take leading cap params; sealed free names require cap first arg at check.

| Sample | Cap | Shape |
|--------|-----|--------|
| `std/os/process.oo` | `&SysCap` | wrappers + `main(sys: &SysCap)` honesty probe |
| `std/os/net.oo` | `&NetCap` | `fetch` / TCP-UDP / M166 slot IO take `net` first |
| `std/os/sync.oo`, `std/os/thread.oo` | `&ThreadCap` | mutex / spawn / join |
| `std/os/fs.oo` | `&FsCap` | read/write/path/size |
| `fixtures/sys_syscall_path_a.oo` | `&SysCap` | `main(sys)` + sealed `sys_epoll_*` first-arg |
| `fixtures/libfloor_tcp_io.oo` | `&NetCap` | `main(net)` + `tcp_*` / `sock_raw` |
| `fixtures/malloc_path_a.oo` | `&AllocCap` | `malloc`/`free` under alloc |
| `fixtures/libfloor_thread_cap.oo` | `&ThreadCap` | spawn + mutex |

**Smokes (path A rails):**

- `scripts/sys_syscall_path_a_smoke.sh` — granted SysCap check/emit/runtime residual; **bare** `sys_epoll_create(0)` refused by `oodac check`
- `scripts/tcp_io_smoke.sh` — seal table honesty; **bare** `tcp_read` refused by `oodac check` when product `oodac` present

Pattern to copy:

1. `pub fn main(sys: &SysCap)` / `main(net: &NetCap)` (or library fn with leading cap param)  
2. Sealed call **cap IDENT first**: `sys_exec(sys, …)`, `tcp_connect(net, …)`  
3. Bare call without cap → `oodac check` non-zero (`E_CAP` / capability)

## M169 path A — cast forgery posture

| Claim | Path A status |
|-------|----------------|
| Check default-deny sealed free names without `&…Cap` param | **In** (e.g. bare `dlopen` → E_CAP) |
| Runtime zero / classic magic-int forge deny | **In** (Backend-C `oo_cap_require`) |
| Product `as fn(&UnsafeFFICap, …)` cast syntax | **Not a product surface** — `as` is reserved IDENT; no KW_AS cast form in emit/check |
| AGY reported cast-bypass | **Residual if/when cast surface lands** — not proven closed against future `as`/coerce syntax |

**Residual (do not claim closed):** full cryptographic / unforgeable object-caps; any future cast/coerce surface must re-seal under check. Classic magic-int forge deny remains the claimed Backend-C floor.

---

## Related

- `bootstrap/CAPS_MATRIX.md`  
- `bootstrap/CAP_FFI.md` — Cap vs FFI residual (PM 6.3 / M25); process-local seals ≠ FFI sandbox  
- `runtime/chs_rt_sys.c`, `chs_rt_fs.c`, `chs_rt_time_rand.c`, `chs_rt_alloc.c`  
- `oodac/check_caps.oo`, `check_cap_util.oo`, `c_emit_fn.oo`, `c_emit_lower.oo`  
