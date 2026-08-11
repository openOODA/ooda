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


## Path A product floor (alpha) — M153

**Path A marker:** `CONCURRENCY_PATH_A_ALPHA`  
**Status:** path A **In** — check default-deny of named residual free calls (`check_residual.oo`).  
**In:** channel_new/actor_spawn free calls refused at check  
**Sealed residual:** `thread_spawn`/`mutex_lock`/`mutex_unlock` under `&ThreadCap` → runtime `Result` **Err** path A (no OS threads/pthreads/spinlocks)  
**Dual path:** granted `thread` → residual Err message; zero/magic forge → `ERR\tcap\t…` + exit  
**Rails:** `scripts/residual_path_a_floor_smoke.sh`, `scripts/libfloor_mutex_thread_smoke.sh`, `scripts/libfloor_thread_gpu_smoke.sh`  
**Still residual:** full DESIGN fearless concurrency / real OS threads (not claimed).

## Rails

- Doc marker: `CONCURRENCY_RESIDUAL_ALPHA`
- Smoke: `scripts/concurrency_residual_smoke.sh`, `scripts/libfloor_mutex_thread_smoke.sh`
- Fixture: `fixtures/concurrency_marker.oo` (marker); `fixtures/libfloor_mutex.oo`, `fixtures/libfloor_thread_spawn.oo`, `fixtures/libfloor_thread_cap.oo`
- Std: `std/os/sync.oo`, `std/os/thread.oo` (ThreadCap residual wrappers)

## Next (path A, not this pack)

Real OS threads / mutexes under ThreadCap — still not full DESIGN depth.
