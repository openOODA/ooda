# Memory quotas — path A product floor (alpha)

**Status:** PM **3.3 → done (alpha)** for process-local AllocCap helpers + ambient List quota.  
**M166:** product free-name aliases `malloc` / `free` / `realloc` under `&AllocCap` (same rails as `alloc_bytes` / `free_bytes`).

**Product floor (path A In under AllocCap):**
- `alloc_bytes(alloc, n) -> Int` / **`malloc(alloc, n) -> Int`** → `oo_alloc_bytes` (smoke size token; raises ambient List ceiling)
- `free_bytes(alloc, p)` / **`free(alloc, p)`** → `oo_free_bytes` (cap re-check; handle free is no-op vs real OS heap)
- **`realloc(alloc, p, n) -> Int`** path A: free then alloc (quota token adjust; **not** OS `realloc` / pointer stability)
- `OO_LIST_AMBIENT_QUOTA` fail-closed for List growth; raise via `alloc_bytes` / `malloc`

**Honesty residual (do not claim):**
- **Not** OS `rlimit` / cgroup memory isolation  
- **Not** GC / automatic reclamation  
- **Not** typed `&AllocCap<N>` or full heap sandbox / ASAN  
- **Not** raw ambient libc `malloc` without AllocCap  

**Rails:** `memory_quota_product_floor_smoke.sh` (`alloc_cap_smoke` + `list_quota_smoke`) + `scripts/malloc_path_a_smoke.sh`.
