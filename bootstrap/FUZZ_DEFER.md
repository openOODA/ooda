# `ooda test --fuzz` — Contract Fuzzer Honesty

**Status (v0.183.0-alpha):** CLI **un-gated**. Implementation is a **Python residual**, not a pure-`.oo` native fuzzer.

## Product path (honest)

```
ooda test <file.oo> --fuzz [iterations]
  → scripts/ooda_product.sh test
  → scripts/ooda_test_verify.sh (OODA_TEST_FUZZ*)
  → python3 scripts/ooda_test_harness.py (+ ooda_fuzz_scan.py / ooda_fuzz_emit.py)
  → Backend-C harness build + exec
```

| Claim | Reality |
|-------|---------|
| CLI accepts `--fuzz` | Yes |
| Pure-`.oo` fuzzer in `oodac` | **No** — `fuzz_gen.oo` is dead (not imported) |
| Zero-Python product path | **No** — harness is Python |
| Safe for all contract shapes | **No** — residual; string ARC / bool emit can still break builds |

## Residuals (do not market as sealed)

1. **Python harness** — not self-hosted pure path  
2. **M2 ARC interaction** — string concat / reassign under harness still hostile  
3. **`fuzz_gen.oo`** — orphan; not on product path  
4. Self-host **`PURE_NO_ARC`** may affect harness emit stability when using tree `oodac`

## When to claim “native”

Only when **all** hold:

1. Pure path zero-Rust; modules ≤256; no Python on fuzz path  
2. Explicit domain (e.g. pure `Int` params) with pass **and** fail rails  
3. Cap-aware sandbox for effectful fixtures  
4. `FUZZ_DEFER.md` updated and honesty probes match

## Interim agent guidance

- Use `ooda test <file.oo>` for verify / `assert_eq!`  
- Treat `--fuzz` as **MVP Python residual**, not beta surface  
- Do not document exit-2 residual for `--fuzz` (un-gated)  

## Related

- `scripts/ooda_test_verify.sh` — real test + fuzz entry  
- `scripts/ooda_fuzz_*.py` — residual harness  
- `qa/probe_honesty_tests.sh` — K2 expects un-gated + residual honesty  
- DESIGN § self-testing / fuzz (north star; not auto-promoted to beta)
