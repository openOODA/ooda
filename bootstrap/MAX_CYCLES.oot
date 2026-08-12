# MaxCycles (CPU quota) — product floor (alpha) + residual DESIGN extras

**Marker:** `MAX_CYCLES_RESIDUAL_ALPHA`  
**Status:** **Product floor In (alpha).** PM **3.4** → **done (alpha)** for path A.  
DESIGN extras (`#[MaxCycles]`, OS cgroup, static WCET) remain residual.

## Product floor (In — production-ready alpha)

| Form | Status |
|------|--------|
| `// MAX_CYCLES: N` with N > 0 (multi-digit OK) | **In** — while + range-`for` + call-entry/recursion on shared static `__oo_mc` |
| Exceed budget | **In** — `ERR\tmax_cycles\texceeded` + non-zero exit |
| `// MAX_CYCLES: 0` / invalid N | **Fail-closed** (no silent disable) |
| Shared budget across nested loops / helpers | **In** — file-static counter |

### How fuel is applied

1. Parse `// MAX_CYCLES: N` → `#define OO_MC_LIMIT N` + static `__oo_mc`  
2. Debit each `while` body, range-`for` body, and **function entry** (incl. recursion)  
3. Optional inject rail if native emit lacks counters (`max_cycles_fuel_inject.sh`)

```
emit-c <file.oo> → gcc + chs_rt → run
```

## Residual (not product floor)

| Form | Status |
|------|--------|
| `#[MaxCycles(N)]` attribute grammar | **residual** |
| OS cgroup / cpulimit / RLIMIT_CPU | **residual** (not OS isolation) |
| Static WCET that refuses unbounded loops at compile time | **residual** |
| Non-range `for` / every control form | **residual** depth |

**Fail-closed residual:** path A is **not** OS CPU isolation. Files without the marker are not fuel-limited (opt-in).

## What we do **not** claim

- OS **cgroup** / **cpulimit** / scheduler isolation  
- OS **rlimit** CPU time as MaxCycles  
- Static WCET proof  
- Full DESIGN hybrid metering / attribute syntax  

## Rails (must stay green)

- `scripts/max_cycles_enforce_smoke.sh` — while pass/fail/zero  
- `scripts/max_cycles_for_enforce_smoke.sh` — range-for  
- `scripts/max_cycles_shared_smoke.sh` — shared budget  
- `scripts/max_cycles_recursion_smoke.sh` — recursion  
- `scripts/max_cycles_multi_digit_smoke.sh` — N≥10 emit  
- `scripts/max_cycles_residual_smoke.sh` — honesty  
- `ci_product.sh` wires the above (multi-digit added if missing)
