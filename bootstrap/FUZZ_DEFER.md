# `ooda test --fuzz` — Contract Fuzzer Honesty

**Status (M3 Int + M10 Bool + M13 String + M16 List + M46/M49 Int multi-arg):** CLI **un-gated**. Pure bash path for **`// FUZZ_DOMAIN: int|bool|string|list`**. Int depth + Bool + String + List pass/fail **CI-wired**. Int **arity-2/3 multi-arg In**; arity≥4 / multi-arg non-int still fail-closed residual.

## Product path (honest)

```
ooda test <file.oo> --fuzz [iterations]
  → scripts/ooda_product.sh test
  → scripts/ooda_test_verify.sh (OODA_TEST_FUZZ*)
  → scripts/ooda_fuzz_pure.sh     # pure domain markers (no python3)
  → emit-c + bash ARC decl inject + gcc + chs_rt
  → exec harness
```

| Claim | Reality |
|-------|---------|
| CLI accepts `--fuzz` | Yes |
| Python on `--fuzz` critical path | **No** |
| Domain | **`// FUZZ_DOMAIN: int\|bool\|string\|list`** (+ optional `list_int` → list) + TARGET/REQUIRES/ENSURES markers |
| Multi `// FUZZ_ENSURES` | AND-combined (not last-wins) |
| Depth rails | int: add/mul/abs/clamp (`fuzz_int_depth_smoke.sh`); bool/string/list: domain pass + fail (`fuzz_*_smoke.sh`) |
| Multi-arg Int | **arity-2/3 In** (`__fuzz_x`/`y`/`z`, `f(x[,y[,z]])`); markers use `x`/`y`/`z`/`result` |
| Pure-`.oo` fuzzer in `oodac` | **No** — `fuzz_gen.oo` still orphan |
| Arity ≥4 / multi-arg non-int | **Fail closed** |

## Fixtures

- `fixtures/fuzz_int_domain.oo` — int pass rail
- `fixtures/fuzz_int_add.oo` / `mul` / `abs` / `clamp` — int depth pass
- `fixtures/fuzz_int_fail.oo` — int fail rail
- `fixtures/fuzz_int_multi_add.oo` / `fuzz_int_multi_fail.oo` — Int arity-2 pass/fail
- `fixtures/fuzz_int_multi3_add.oo` / `fuzz_int_multi3_fail.oo` — Int arity-3 pass/fail
- `fixtures/fuzz_int_multi_arity4.oo` — arity≥4 fail-closed residual
- `fixtures/fuzz_int_multi_weak.oo` / `fuzz_bool_multi_weak.oo` — weak multi-arg fail-closed
- `fixtures/fuzz_bool_domain.oo` / `fuzz_bool_fail.oo` — bool pass/fail
- `fixtures/fuzz_string_domain.oo` / `fuzz_string_fail.oo` — string pass/fail
- `fixtures/fuzz_list_domain.oo` / `fuzz_list_fail.oo` — list pass/fail (`List[Int]`, length bounds; elements fixed [-8,16])

## Residuals

1. Arity ≥4 pure path fail-closed; multi-arg non-int fail-closed
2. AST-parsed contracts (markers only today)
3. `fuzz_gen.oo` orphan
4. ~~Verify path without `--fuzz` Python harness~~ **closed M50** — `ooda_verify_pure.sh` (bash+awk; no python3 / pure_build)

## Related

- `scripts/ooda_fuzz_pure.sh` + `ooda_fuzz_pure_gens.sh`
- `scripts/fuzz_int_depth_smoke.sh` / `fuzz_bool_smoke.sh` / `fuzz_string_smoke.sh` / `fuzz_list_smoke.sh` / `fuzz_multi_arg_smoke.sh` (in `ci_product`)
- `bootstrap/MULTI_ARG_FUZZ.md` — arity-2/3 In; arity≥4 residual honesty
