# Capability seals (static + runtime)

**Status:** product truth on pure Backend-C path.  
**Product rule:** claim only what is implemented — static check **and** runtime magic-token re-check for sealed FS/Sys/Env ops.

---

## What is true today

| Layer | Behavior |
|-------|----------|
| **Check** | `oodac/check_caps.oo` — default-deny sealed free/method names; require matching `&FsCap` / `&SysCap` / `&EnvCap` / `&NetCap` param |
| **Emit (Backend-C)** | Cap params lower to `long long`; sealed calls **pass the cap as first arg**; `main` injects `OO_CAP_FS` / `OO_CAP_SYS` / `OO_CAP_ENV` magic tokens |
| **Runtime (`chs_rt`)** | `oo_cap_require(got, want, op)` gates `read_file` / `write_file` / `path_exists` / `file_size` / `env_get`; preamble `oo_sys_exec1` same for Sys |
| **Native binary** | Forged or zero cap → `ERR\tcap\t…` + exit 1 (not ambient I/O) |

Security for sealed I/O on the claimed path:

1. **Compile-time refuse** — missing cap param → check fail  
2. **Runtime seal** — wrong token → `oo_cap_require` exit  
3. **Net** — still emit residual (no product network)

Magic tokens (must match emit preamble + `runtime/chs_rt_fs.c`):

| Cap | Value |
|-----|-------|
| `OO_CAP_FS` | `0x4F4F4653` (`OOFS`) |
| `OO_CAP_SYS` | `0x4F4F5359` (`OOSY`) |
| `OO_CAP_ENV` | `0x4F4F454E` (`OOEN`) |
| `OO_CAP_NET` | `0x4F4F4E54` (`OONT`) — check only; no product runtime |

These are **not** cryptographic object-caps. They are process-local magic integers injected into `main`. They stop accidental/forged ambient calls when code is lowered through Backend-C; they do not stop a hostile hand-edited binary that hardcodes the magic constant.

---

## What we do **not** claim

- Cryptographic / unforgeable object capabilities across process trust boundaries  
- Interpreter-style dynamic capability attenuation graphs  
- Net product I/O (fail-closed residual — see `CAPS_MATRIX.md`)  
- Multi-arg `sys_exec` full argv (product is `oo_sys_exec1` last/single cmd)

---

## Pointers

| Path | Role |
|------|------|
| `bootstrap/CAPS_MATRIX.md` | Op matrix + runtime seal |
| `oodac/check_caps.oo` | Static seal |
| `oodac/c_emit_lower.oo` | Pass cap args; net residual |
| `oodac/c_emit_preamble.oo` | `OO_CAP_*` + `oo_cap_require` + `oo_sys_exec1` |
| `oodac/c_emit_fn.oo` | `main` injects magic tokens |
| `runtime/chs_rt_fs.c` | Cap-checked FS/env |
| `scripts/caps_matrix_smoke.sh` | Check + emit + runtime + forge deny |

## Audit closures (2026-08)

| Hole | Closure |
|------|---------|
| Magic-int forge via pure multi emit | Emit requires bare cap **IDENT** first arg (`c_arg_is_cap_ident`); int/lit rejected |
| Product single-file build skip check | `oodac_pure_build` runs full `check` when module count is 1 |
| Torn `write_file` success | `fwrite`/`ferror`/`fclose` checked; `/dev/full` → Err |
| Incomplete `let x =` | Emit `ERR\tc_emit\tincomplete let RHS` |
| Hostile multi-MB / large garbage | Per-file **64KiB** gate at load; expanded ≤1MiB at check |
| Assign-form match silent `.val` | Full arm lower (`c_emit_match_assign`) |
| `unwrap` on `OoResV` | Env kind V → empty Ok / `ERR\tunwrap` on Err |

**Still residual (honest):** multi-module pure_build does **not** run full expanded typecheck on oodac-scale sources (hang budget); emit-level cap IDENT seal still applies. Magic tokens remain forgeable by hand-editing a binary to hardcode `OO_CAP_*`. `sys_exec` remains `system(3)` shell with SysCap.
