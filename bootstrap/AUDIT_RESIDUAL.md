# Audit residual (honest, post-audit closures)

**Purpose:** List attack surfaces that remain after AUDIT.md runs on openOODA.  
**Not a beta claim.** Do not market these as sealed.

---

## Closed (do not re-open without new evidence)

| ID | Item |
|----|------|
| C1 | Magic-int forge on product build / emit |
| C2 | `/dev/full` torn `write_file` success |
| H1/H2 | unwrap OoResV; assign-form match |
| H5/H6 | incomplete let; per-file size gate |
| F01 | Check arg-flow: Fs/Sys/Env must pass cap IDENT or method receiver |

---

## Open residual (by design or cost)

| ID | Residual | Mitigation today | Close when |
|----|----------|------------------|------------|
| **R1** | Magic tokens forgeable by hand-editing native binary | Documented process-local seal | Crypto object-caps (out of beta) |
| **R2** | `sys_exec` → `system(3)` full shell with SysCap | CLI path quotes via `shell_sq`; product programs with SysCap can shell | `execve` argv floor |
| **R3** | Multi-arg `sys_exec` drops middle args; product is last-arg | Documented; last-arg split is **string-aware** (commas in `"..."` kept) | Full argv ABI |
| **R4** | Large multi-module pure_build (`nmods>8`) skips full expanded `check` | Emit cap IDENT seal; small multi (≤8) runs check | Faster typecheck or cap-only check pass |
| **R5** | Expanded typecheck can hang/cost on oodac-scale trees | Per-file 64KiB load; expanded 1MiB; pure_build timeout | Incremental check |
| **R6** | No `OoStr` / list free — process-lifetime arena | Short-lived CLI processes | Explicit free/arena API |
| **R7** | List-by-value + `realloc` alias UAF if two live list values share `.data` | Emit reassigns single owner in common patterns | Unique/owned list type |
| **R8** | Dynamic/computed sealed callees not scanned by check | IDENT+LPAREN only | Full call-graph check |
| **R9** | Net: check param only; emit residual (no product runtime) | Fail-closed emit | Net product design |

---

## E-M / line lock

- **MAX_LINES=256** on owned `.oo` `.c` `.h` `.sh` **and** product `scripts/*.py`.  
- Python outline/test helpers split into modules under the cap.

---

## Temp lifecycle

- `oodac_pure_build.sh`: `trap` cleans `$TMP` on all exits.  
- `ooda_product.sh` run: `trap` removes temp binary.  
- `ooda_test_verify.sh`: `trap` removes harness/bin unless `OODA_TEST_KEEP=1`.

---

*Revisit when DESIGN adds object-caps, argv exec, or GC. Prefer residual over silent soft-pass.*
