# `ooda test --fuzz` — path A product floor (alpha) + residual

**Marker:** path A pure-domain fuzzer.  
**Status:** **Path A product floor In (alpha).** PM **3.6** → **done (alpha)** for marker pure fuzz.  
**Residual listed:** AST `requires`/`ensures` (not markers), mixed-type multi-arg, other domains, `fuzz_gen.oo` in-compiler, shrink/JSON min-cex, hive-mind — not soft-pass.

## Product path (In — alpha floor)

```
ooda test <file.oo> --fuzz [iterations]
  → scripts/ooda_product.sh test
  → scripts/ooda_fuzz_pure.sh   # pure domains; no python3
  → emit-c + gcc + chs_rt → harness
```

| Claim | Reality |
|-------|---------|
| CLI `--fuzz` | **In** |
| Python on critical path | **No** |
| Domains | `// FUZZ_DOMAIN: int\|bool\|string\|list` |
| Single-arg pass/fail | **In** (CI: `fuzz_*_smoke`) |
| Int depth (add/mul/abs/clamp) | **In** |
| Homogeneous multi-arg | **In** (Int a2/a3/a≥4; Bool/String/List multi a2+) |
| Mixed-type multi params | **Fail-closed residual** |
| Markers TARGET/REQUIRES/ENSURES | **In** (not full AST contracts) |

## Residual (not product floor)

- Real AST `requires` / `ensures` parsing for fuzz (markers only today)  
- Pure `.oo` fuzzer inside `oodac` (`fuzz_gen.oo` orphan)  
- Struct/float/Result/… domains  
- Mixed-type multi-arg generation  
- Shrink + JSON min counterexample  
- Global hive-mind fuzz (PM 2.4)  

## Rails (must stay green)

- `scripts/fuzz_int_depth_smoke.sh`  
- `scripts/fuzz_bool_smoke.sh`  
- `scripts/fuzz_string_smoke.sh`  
- `scripts/fuzz_list_smoke.sh`  
- `scripts/fuzz_multi_arg_smoke.sh`  
- `scripts/fuzz_product_floor_smoke.sh` (umbrella)  

## What we do **not** claim

- Full DESIGN continuous multi-type contract fuzzer  
- Formal SMT / complete property proof  
- Cap-sandboxed effectful fuzz  
