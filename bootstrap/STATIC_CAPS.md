# Capability seals (static + runtime)

**Status:** product truth on pure Backend-C path (M8 + M12 Time/Rand + M17 Alloc matrix rails).  
**Product rule:** claim only process-local magic-token re-check — **not** cryptographic object-caps.

---

## What is true today

| Layer | Behavior |
|-------|----------|
| **Check** | `oodac/check_caps.oo` — default-deny sealed free/method names; require matching `&FsCap` / `&SysCap` / `&EnvCap` / `&NetCap` / `&TimeCap` / `&RandCap` / `&AllocCap` **param** |
| **Emit (Backend-C)** | Cap params → `long long`; sealed calls pass **cap IDENT first**; `main` injects **`oo_cap_grant_fs/sys/env/net/time/rand/alloc()`** (process-local tokens from entropy) |
| **Runtime (`chs_rt`)** | `oo_cap_require_*` before `read_file` / `write_file` / `path_exists` / `file_size` / `env_get` / `sys_exec` / `fetch` / `now_ms` / `sleep_ms` / `random` / `seed` / `alloc_bytes` / `free_bytes` |
| **Native binary** | Zero or classic fixed magic (`0x4F4F4653` / `…TM` / `…RN` / `…AL` etc.) as “grant” → `ERR\tcap\t…` + exit 1 |

Security for sealed effects on the claimed path:

1. **Compile-time refuse** — missing / wrong cap param → check fail  
2. **Runtime seal** — wrong token → `oo_cap_require` exit  
3. **Net** — `fetch` product-lowered + runtime (`oo_fetch`); other net names may still residual at emit  
4. **Time / Rand** — `now_ms` / `sleep_ms` need `&TimeCap`; `random` / `seed` need `&RandCap` (process-local seal only)  
5. **Alloc** — `alloc_bytes` / `free_bytes` need `&AllocCap` (process-local seal only; **not** OS rlimit / heap isolation)

**Honest ceiling:** process-local tokens stop accidental ambient effects (I/O, clock, entropy, explicit alloc helpers) and classic magic forges on Backend-C. They do **not** stop a hostile binary that calls `oo_cap_grant_*` or patches `oo_cap_require` out.

Classic `0x4F4F*` constants are **forged values that must be denied**, not ambient grants.  
`oo_random` uses host entropy when available (else process LCG) — **not** a cryptographic CSPRNG guarantee.

**Ambient residual (intentional):** `list_new` / `list_push` / string concat stay free for alpha — sealing them would brick the pure compiler and fixtures. Only the narrow `alloc_bytes` / `free_bytes` surface is sealed under AllocCap.

---

## What we do **not** claim

- Cryptographic / unforgeable object capabilities  
- Biometric or OS-level caps  
- Full net surface beyond `fetch`  
- Cryptographically secure randomness or attested clocks  
- Heap sandboxing, ASAN, or OS `rlimit` isolation for AllocCap  
- Ambient effects without an explicit cap param on the product path  
- **C FFI / `dlopen` / raw-pointer / Compile-Time FFI seal** — process-local caps do **not** seal C interop; `&UnsafeFFICap` is residual-only (see [`CAP_FFI.md`](CAP_FFI.md))  

---

## Rails

`scripts/caps_matrix_smoke.sh` + `scripts/alloc_cap_smoke.sh` (in `ci_product`):

- Static pass/fail corpus for FS/Sys/Env/Net/Time/Rand/Alloc  
- Product `ooda check` deny ×7 families (incl. `no_cap_now_ms`, `no_cap_random`, `no_cap_alloc_bytes`)  
- Runtime pass: Fs roundtrip, path_exists, Env, Sys argv, Time `now_ms`, Rand `random`, Alloc `alloc_bytes`  
- Runtime forge deny: Fs/Sys/Env/Net/Time/Rand/Alloc (zero + classic magic)  
- Emit fail-closed: fetch without NetCap arg; `now_ms` without TimeCap IDENT; `alloc_bytes` without AllocCap IDENT; magic-int forge build  

---

## Related

- `bootstrap/CAPS_MATRIX.md`  
- `bootstrap/CAP_FFI.md` — Cap vs FFI residual (PM 6.3 / M25); process-local seals ≠ FFI sandbox  
- `runtime/chs_rt_sys.c`, `chs_rt_fs.c`, `chs_rt_time_rand.c`, `chs_rt_alloc.c`  
- `oodac/check_caps.oo`, `check_cap_util.oo`, `c_emit_fn.oo`, `c_emit_lower.oo`  
