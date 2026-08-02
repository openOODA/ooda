# v0.177.0-alpha

**Tool:** Assembly depth to B0 (primary); Honesty budget (secondary)  
**Shipper:** Grok 4.5 (xAI)  
**RS_COUNT:** 28 (flat — no `.rs` deleted; pure CHS path no longer forces libooda)

## Bootstrap / assembly depth

- **Pure CHS → C native link** uses **gcc + `runtime/chs_rt.c` only** when the program does not call host FFI (`chs_build`, `host_ast_dump`, `host_check`, `host_token_dump`).
- Host FFI programs still require `libooda.a` and **fail closed** if the staticlib is missing.
- Generated C omits host decls for pure programs; `chs_rt.c` gates host wrappers behind `OODA_WITH_HOST_FFI`.

## Honesty

- No silent link of optional staticlib for host-using programs (was soft-skip).

## Non-claims

- Not zero-Rust beta; oodac still hybrid (`chs_build` host); RS_COUNT unchanged.
- Fixed-point stage-1 oodac native may still fail on sealed `read_file` (pre-existing).
