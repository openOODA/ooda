# `ooda test --fuzz` — Contract Fuzzer Honesty

**Status (M3 pure Int domain):** CLI **un-gated**. Product `--fuzz` critical path is **pure bash** (`ooda_fuzz_pure.sh`) for **`// FUZZ_DOMAIN: int`** fixtures only — **no Python** on that path. Broader contract shapes remain residual.

## Product path (honest)

```
ooda test <file.oo> --fuzz [iterations]
  → scripts/ooda_product.sh test
  → scripts/ooda_test_verify.sh (OODA_TEST_FUZZ*)
  → scripts/ooda_fuzz_pure.sh     # pure Int domain markers (no python3)
  → emit-c + bash ARC decl inject + gcc + chs_rt  (no oodac_pure_build / no python3)
  → exec harness
```

| Claim | Reality |
|-------|---------|
| CLI accepts `--fuzz` | Yes |
| Python on `--fuzz` critical path | **No** — bash harness + bash/gcc link (no `python3`, no `ooda_fuzz_*.py`, no `ooda_test_harness.py`) |
| Domain | **`// FUZZ_DOMAIN: int`** + `// FUZZ_TARGET` / `// FUZZ_REQUIRES` / `// FUZZ_ENSURES` markers |
| Pure-`.oo` fuzzer in `oodac` | **No** — `fuzz_gen.oo` still orphan |
| All contract shapes | **No** — non-int-domain sources **fail closed** with clear ERR |
| Emitter | Prefer `bootstrap/seed/oodac` for multi-fn harness stability; tree `oodac` fallback |

## Pure Int domain (shipped)

Markers (fixture-authored; bash does not parse contracts from AST):

- `// FUZZ_DOMAIN: int` — required gate
- `// FUZZ_TARGET: <fn> <min> <max>` — single `Int` param range (LCG sample)
- `// FUZZ_REQUIRES: <fn> <expr with x>` — skip when false
- `// FUZZ_ENSURES: <fn> <expr with x and/or result>` — `process_exit(1)` when false

Fixtures:

- `fixtures/fuzz_int_domain.oo` — pass rail (`abs_nonneg`)
- `fixtures/fuzz_int_fail.oo` — fail rail (`always_bad`, `ensures result > x` but returns `x`)

## Residuals (do not market as sealed)

1. **Non-int domain** — String/Bool/List/multi-arg still unsupported (fail closed; no Python fallback)
2. **M2 ARC interaction** — not exercised by pure Int fixtures
3. **`fuzz_gen.oo`** — orphan; not on product path
4. **Verify path** (`ooda test` without `--fuzz`) may still use Python harness for `assert_eq!` lowering

## When to claim “full native fuzzer”

Only when **all** hold:

1. Pure path zero-Rust; modules ≤256; no Python on fuzz path *(Int domain: done)*
2. Explicit domain with pass **and** fail rails *(Int: done)*
3. Cap-aware sandbox for effectful fixtures
4. Broader types / multi-param without Python residual
5. Honesty probes match current docs

## Interim agent guidance

- Use `ooda test <file.oo>` for verify / `assert_eq!` (may use Python harness)
- Use `ooda test fixtures/fuzz_int_domain.oo --fuzz N` for pure Int contract fuzz
- Non-marker sources under `--fuzz` → fail-closed ERR (not residual exit-2 gate on CLI)
- Do not document exit-2 residual for `--fuzz` CLI acceptance (un-gated)

## Related

- `scripts/ooda_fuzz_pure.sh` — pure Int harness generator
- `scripts/ooda_test_verify.sh` — real test + fuzz entry
- `scripts/ooda_fuzz_*.py` / `ooda_test_harness.py` — **not** on `--fuzz` path (verify residual only)
- DESIGN § self-testing / fuzz (north star; not auto-promoted to beta)
