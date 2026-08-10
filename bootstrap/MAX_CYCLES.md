# MaxCycles (CPU quota) — path A while fuel + residual

**Marker:** `MAX_CYCLES_RESIDUAL_ALPHA`  
**Status:** **M48/M54/M58 + M126 recursion In** — Backend-C `while` + range-`for` + **call-entry** fuel under `// MAX_CYCLES: N`.  
PM **3.4**.

## Product surface (In — not names-only)

| Form | Status |
|------|--------|
| `// MAX_CYCLES: N` (N > 0) | **In:** while + range-`for` + **fn call-entry** on shared static `__oo_mc`; exceed → `ERR\tmax_cycles\texceeded` |
| `// MAX_CYCLES: 0` / invalid N | **Fail-closed** (no silent disable) |
| `#[MaxCycles(N)]` | Named residual only (no attribute grammar / no static WCET proof) |

### How fuel is applied

1. **Native emit:** parse marker → `__mc__` env → file-level `static long long __oo_mc`  
2. **Debits:** each `while` body, range-`for` body, and **function entry** (incl. recursive helpers)  
3. **Inject rail** (`scripts/max_cycles_fuel_inject.sh`): while fuel if native emit lacks counters  

Not OS cgroup / non-range for / static WCET / `#[MaxCycles]`.
```
emit-c <file.oo>  [→ inject if no __oo_mc]  → gcc + chs_rt → run
```

## What is true today

| Layer | Behavior |
|-------|----------|
| **Parse / check** | Comment marker; no `#[MaxCycles]` grammar |
| **Emit (Backend-C)** | Path A while + path B range-for fuel (native; while inject rail still available) |
| **Runtime** | Exceed → `ERR\tmax_cycles\texceeded` + exit 1 |
| **Honesty** | Residual below; **do not** claim names-only after path A/B In |

**Fail-closed residual:** path A/B is **not** OS CPU isolation. Code without the marker is not fuel-limited. Marker is opt-in.

## What we do **not** claim

- OS **cgroup** / **cpulimit** / scheduler isolation  
- OS **rlimit** CPU time as MaxCycles  
- Static WCET proof that refuses unbounded loops at compile time  
- Full hybrid fuel metering (DESIGN / RP-3-4 goal)  
- Fuel on recursion / non-range `for` / other control  
- `#[MaxCycles(N)]` attribute enforcement  

## Rails

- Doc marker: `MAX_CYCLES_RESIDUAL_ALPHA`
- Product: `scripts/max_cycles_enforce_smoke.sh` + `scripts/max_cycles_smoke.sh`
- Residual honesty: `scripts/max_cycles_residual_smoke.sh`
- Inject helper: `scripts/max_cycles_fuel_inject.sh`
- Fixtures: `max_cycles_pass.oo`, `max_cycles_fail.oo`, `max_cycles_zero.oo`, `max_cycles_marker.oo`
