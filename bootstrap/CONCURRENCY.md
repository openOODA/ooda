# Fearless concurrency

**Status:** residual honesty (not enforced). PM **5.3**.  
**Marker:** `CONCURRENCY_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: message-passing concurrency + caps (DESIGN)

## What is true today

Product alpha does **not** ship this DESIGN surface as enforced. Process-local caps / pure self-host / Backend-C floor remain the claimed path. This residual names the gap so agents do not treat aspiration as shipped.

## Fail-closed residual

Do **not** treat the named surface as a security or product boundary. Absence of implementation is residual, not silent green.

## What we do **not** claim

- No fearless concurrency runtime product path shipped
- No actor model, no channels product, no thread pools, no shared-memory product
- No DESIGN-depth “fearless concurrency” claim


## Path A product floor (alpha) — M153 / M162 / M163

**Path A marker:** `CONCURRENCY_PATH_A_ALPHA`  
**Status:** path A **In** — check default-deny of named residual free calls (`check_residual.oo`); joinable threads under ThreadCap (M163).  

**In (path A):**
- `channel_new` / `actor_spawn` free calls refused at check
- Real `pthread` **mutex** under `&ThreadCap`: `mutex_lock` / `mutex_unlock` → `Result` Ok
- Real **joinable** `pthread_create` under `&ThreadCap`: `thread_spawn(thread, name) -> Result[String,String]` returns Ok(`"tid:N"`) slot handle (not detach-by-default)
- `thread_join(thread, slot: Int) -> Result[String,String]` joins slot N; runtime also parses `"tid:N"` via `oo_thread_join_s`
- Dual path: granted `thread` → product Ok; zero/magic forge → `ERR\tcap\t…` + exit; bare call without ThreadCap refused at check

**Still residual:**
- Channels / actors / message-passing DESIGN surface
- Thread pools, shared memory product, fearless-concurrency DESIGN
- GpuCap `gpu_launch` remains seal-only residual Err
- No full scheduler / join-by-name product beyond slot table

**Rails:** `scripts/residual_path_a_floor_smoke.sh`, `scripts/libfloor_mutex_thread_smoke.sh`, `scripts/libfloor_thread_gpu_smoke.sh`, `scripts/thread_join_smoke.sh`

## Rails

- Doc marker: `CONCURRENCY_RESIDUAL_ALPHA`
- Smoke: `scripts/concurrency_residual_smoke.sh`, `scripts/libfloor_mutex_thread_smoke.sh`, `scripts/thread_join_smoke.sh`
- Fixture: `fixtures/concurrency_marker.oo` (marker); `fixtures/libfloor_mutex.oo`, `fixtures/libfloor_thread_spawn.oo`, `fixtures/libfloor_thread_cap.oo`, `fixtures/thread_join.oo`
- Std: `std/os/sync.oo`, `std/os/thread.oo` (ThreadCap wrappers; honesty may lag runtime product)
- Runtime: `runtime/chs_rt_libfloor.c` (mutex/gpu), `runtime/chs_rt_thread.c` (joinable spawn/join)

## Next (path A, not this pack)

Channels residual; actor model residual — still not full DESIGN depth.
