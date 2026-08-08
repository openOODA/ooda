# Audit residual (honest, post-audit closures)

**Purpose:** List attack surfaces that remain after AUDIT.md runs on openOODA.  
**Not a beta claim.** Do not market these as sealed.

---

## Closed (do not re-open without new evidence)

| ID | Item | Evidence (one-liner) |
|----|------|----------------------|
| C1 | Magic-int forge on product build / emit | emit-c + check reject `write_file(1330595411,…)`; `caps_matrix_smoke` forge build denied |
| C2 | `/dev/full` torn `write_file` success | product emit+run → `TORN_ERR` (not Ok) |
| H1/H2 | unwrap OoResV; assign-form match | prior fixture rails |
| H5/H6 | incomplete let; per-file size gate | prior fixture rails |
| F01 | Check arg-flow: Fs/Sys/Env must pass cap IDENT or method receiver | check corpus deny/allow |
| **R1** | Fixed published magic caps (`0x4F4F4653` etc.) | process-local `oo_cap_grant_*`; runtime hard-exit `ERR cap` on magic/0 forge |
| **R2** | `sys_exec` → `system(3)` shell | product emit → `oo_sys_exec` only; runtime `fork`+`execvp` (no `system(` in `chs_rt_sys.c`) |
| **R3** | Multi-arg `sys_exec` drops middle args | emit full `(OoStr[]){…}` argv; product `sys_exec(sys,"sh","-c","echo multi")` ok |
| **R7** | List-by-value `realloc` alias UAF | `chs_rt_list.c`: push always fresh buffer + free prior; runtime list smoke ok |
| **R9** | Net: no product runtime fetch | product `fetch(net, http://127.0.0.1:…)` returns body via sockets HTTP/1.0 GET |

---

## Open residual (by design or cost)

| ID | Residual | Mitigation today | Close when |
|----|----------|------------------|------------|
| **R1′** | Process can still self-grant via `oo_cap_grant_*` or patch out `oo_cap_require_*` in-binary | Process-local random tokens; not crypto object-caps | True object-caps / attestation (out of beta) |
| **R4** | Full expanded `check` on oodac-scale trees times out (e.g. `oodac check oodac/main.oo` >120s); bootstrap often needs `PURE_SKIP_CHECK=1` | `oodac_pure_build.sh` **always** runs check unless skip; timeout scales with `nmods` (fail-closed) | Faster typecheck / incremental check; green self-check without skip |
| **R5** | Expanded typecheck cost/hang on large trees | Per-file 64KiB load; expanded 1MiB; pure_build timeout | Incremental check |
| **R6** | No automatic `OoStr` free; list free is manual API (`oo_*list_free`) — process-lifetime arena for strings | Short-lived CLI processes; list free available | Emit-side drop / arena API for strings + lists |
| **R8** | Dynamic/computed sealed callees not scanned by check | IDENT+LPAREN only | Full call-graph check |

---

## E-M / line lock

- **MAX_LINES=256** on owned `.oo` `.c` `.h` `.sh` **and** product `scripts/*.py`.  
- Python outline/test helpers split into modules under the cap.
- **Proven:** `./scripts/check_file_lines.sh --ratchet` → **O=0** (post R1–R9 runtime work).

---

## Temp lifecycle

- `oodac_pure_build.sh`: `trap` cleans `$TMP` on all exits.  
- `ooda_product.sh` run: `trap` removes temp binary.  
- `ooda_test_verify.sh`: `trap` removes harness/bin unless `OODA_TEST_KEEP=1`.

---

## Proof session notes (local)

| Proof | Result |
|-------|--------|
| Emit `sys_exec` → `oo_sys_exec` argv, no `system(` | PASS |
| Product `sys_exec(sys,"true")` + multi-arg | PASS |
| Magic `0x4F4F4653` / forge at check+runtime | PASS |
| Product `fetch` to local HTTP body | PASS |
| Line lock ratchet O=0 | PASS |
| `c_emit_smoke` / `caps_matrix_smoke` / `shell_safety` / `import_load` | PASS |
| Full `oodac check oodac/main.oo` (120s) | **TIMEOUT** — R4 remains open |
| Bootstrap | `PURE_SKIP_CHECK=1` used (honest: full self-check not proven) |

---

*Revisit when DESIGN adds object-caps, incremental check, or GC. Prefer residual over silent soft-pass.*
