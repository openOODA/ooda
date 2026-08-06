# Static caps residual (canonical)

**Status:** honesty residual (not a beta claim; DESIGN.md unchanged).  
**Product rule:** never claim **runtime cap re-check** on native.  
**Canonical residual:** sealed ops are **static-check only** on the pure Backend-C path.

---

## What is true today

| Layer | Behavior |
|-------|----------|
| **Check** | `oodac/check_caps.oo` — default-deny sealed free/method names; require matching `&FsCap` / `&SysCap` / `&EnvCap` / `&NetCap` param |
| **Emit (Backend-C)** | Cap params lower to **`int` placeholders**; leading cap args dropped on sealed calls |
| **Runtime (`chs_rt`)** | FS/env/sys symbols are **ambient libc** (`fopen` / `getenv` / `system`, …) — **no token gate** |
| **Native binary** | **No re-check** of caps at process runtime |

Security for sealed I/O on the claimed path is **compile-time refuse** (missing cap → check fail; net product → emit residual). Once a program checks and links, the binary performs ambient OS I/O for lowered ops.

---

## What we do **not** claim

- Runtime object-caps / re-validation of `FsCap` etc. in the native binary  
- Interpreter-style runtime gates on Backend-C product path  
- Net product I/O (fail-closed residual — see `CAPS_MATRIX.md`)

---

## Pointers

| Path | Role |
|------|------|
| `bootstrap/CAPS_MATRIX.md` | Op matrix + **Runtime: static-only** section |
| `oodac/check_caps.oo` | Static seal |
| `oodac/c_emit_lower.oo` | Drop cap args; net residual |
| `runtime/chs_rt_fs.c` | Ambient FS/env |

*Residual statement only. Expand sealed table via CAPS_MATRIX process; do not invent runtime token machinery without DESIGN + product work.*
