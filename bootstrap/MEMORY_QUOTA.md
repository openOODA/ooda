# Memory quotas — path A product floor (alpha)

**Status:** PM **3.3 → done (alpha)** for process-local AllocCap helpers + ambient List quota.  
**Product floor:** `alloc_bytes` / `free_bytes` sealed under `&AllocCap`; `OO_LIST_AMBIENT_QUOTA` fail-closed for List growth; raise via `alloc_bytes`.  
**Residual:** not OS rlimit; not typed `&AllocCap<N>`; not full heap sandbox / ASAN.  
**Rails:** `memory_quota_product_floor_smoke.sh` (`alloc_cap_smoke` + `list_quota_smoke`).
