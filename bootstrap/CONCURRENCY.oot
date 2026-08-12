# Fearless concurrency

**Status:** residual honesty (not enforced). PM **5.3**.  
**Marker:** `CONCURRENCY_RESIDUAL_ALPHA`

## Named surface only (not product green)

- Named only: full DESIGN message-passing concurrency + caps (DESIGN)

## What is true today

Product alpha does **not** ship full DESIGN fearless concurrency. Process-local ThreadCap mutex / joinable threads / channels / thin actors (path A) exist under process-local caps; full actor model and multi-process message-passing remain residual. This pack names the gap so agents do not treat DESIGN aspiration as shipped.

## Fail-closed residual

Do **not** treat the named DESIGN surface as a security or product boundary. Absence of full actor model / multi-process product is residual, not silent green.

## What we do **not** claim

- No full fearless concurrency DESIGN runtime
- No full actor model product (supervision trees, mailboxes-as-types), no thread pools, no shared-memory product, no multi-process channels
- No DESIGN-depth “fearless concurrency” claim


## Path A product floor (alpha) — M153 / M162 / M163 / M164 / M165

**Path A marker:** `CONCURRENCY_PATH_A_ALPHA`  
**Status:** path A **In** — ThreadCap mutex / joinable threads / process-local channels / thin actors.  

**In (path A):**
- Real `pthread` **mutex** under `&ThreadCap`: `mutex_lock` / `mutex_unlock` → `Result` Ok
- Real **joinable** `pthread_create` under `&ThreadCap`: `thread_spawn(thread, name) -> Result[String,String]` returns Ok(`"tid:N"`) slot handle (not detach-by-default)
- `thread_join(thread, slot: Int) -> Result[String,String]` joins slot N; runtime also parses `"tid:N"` via `oo_thread_join_s`
- Process-local **channels** under `&ThreadCap` (M164):
  - `channel_new(thread) -> Result[String,String]` Ok(`"ch:N"`)
  - `channel_send(thread, slot: Int, msg: String) -> Result` (bounded queue; full → Err)
  - `channel_recv(thread, slot: Int) -> Result` Ok(msg) or Err empty
  - Runtime: `runtime/chs_rt_channel.c` — 16 slots × 8 messages, mutex+condvar, process-local only
- **Thin actors** under `&ThreadCap` (M165) — not full actor model:
  - `actor_spawn(thread, name) -> Result` Ok(`"actor:N"`) — joinable thread + private mailbox slot
  - `actor_send(thread, id: Int, msg: String) -> Result` / `actor_recv(thread, id: Int) -> Result` — thin mailbox wrappers
  - Runtime: `runtime/chs_rt_actor.c` — 16 slots × 8 messages; process-local only
- Dual path: granted `thread` → product Ok; zero/magic forge → `ERR\tcap\t…` + exit; bare call without ThreadCap refused at check

**Still residual:**
- Full actor model, supervision trees, multi-process message-passing, thread pools, shared memory product
- GpuCap `gpu_launch` (M165): noop/cpu fallthrough Ok; device shaders residual Err
- No full scheduler / join-by-name product beyond slot table
- No blocking timeout product on recv (path A non-blocking empty Err)

**Rails:** `scripts/residual_path_a_floor_smoke.sh`, `scripts/libfloor_mutex_thread_smoke.sh`, `scripts/libfloor_thread_gpu_smoke.sh`, `scripts/thread_join_smoke.sh`, `scripts/channel_path_a_smoke.sh`, `scripts/actor_path_a_smoke.sh`

## Rails

- Doc marker: `CONCURRENCY_RESIDUAL_ALPHA`
- Smoke: `scripts/concurrency_residual_smoke.sh`, `scripts/libfloor_mutex_thread_smoke.sh`, `scripts/thread_join_smoke.sh`, `scripts/channel_path_a_smoke.sh`, `scripts/actor_path_a_smoke.sh`
- Fixture: `fixtures/concurrency_marker.oo` (marker); `fixtures/libfloor_mutex.oo`, `fixtures/libfloor_thread_spawn.oo`, `fixtures/libfloor_thread_cap.oo`, `fixtures/thread_join.oo`, `fixtures/channel_roundtrip.oo`, `fixtures/actor_path_a.oo`
- Std: `std/os/sync.oo`, `std/os/thread.oo` (ThreadCap wrappers; honesty may lag runtime product)
- Runtime: `runtime/chs_rt_libfloor.c` (mutex/gpu), `runtime/chs_rt_thread.c` (joinable spawn/join), `runtime/chs_rt_channel.c` (channels), `runtime/chs_rt_actor.c` (thin actors)

## Next (path A, not this pack)

Full actor model residual; multi-process channels residual — still not full DESIGN depth.
