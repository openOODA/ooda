# `ooda test --fuzz` — DESIGN deferral (honest residual)

**Status:** fail-closed residual. `ooda test --fuzz` exits **2** and points here.

**Not claimed:** property-based or `#[auto_fuzz]` runner on the pure product path.

## Why deferred

DESIGN / SPEC describe automated fuzzing (boundary inputs, `#[auto_fuzz]`). The pure product path today is:

1. **`ooda test`** — typecheck + lower `assert_eq!` in `verify` → Backend-C harness build+exec  
2. **No host interpreter** — native build+exec only (`ooda run` permanent path)  
3. **Contracts not runtime-enforced** on Backend-C  

A real fuzzer needs at least one of:

| Prerequisite | Why |
|---|---|
| Pure integer-domain generator + harness emit | Safe only for pure arithmetic `assert_eq` targets |
| Contract/refinement lowering or check-phase oracle | Otherwise fuzz cannot score requires/ensures |
| Cap-aware sandbox for effectful code | Avoid ambient I/O while fuzzing |
| Stable API for `#[auto_fuzz]` (or CLI filter) | Agent-discoverable surface |

Shipping a “fake fuzz” that only re-runs fixed asserts would raise \(W\) (hand-wave) and \(U\) (untested claim). Prefer **exit 2 + this note**.

## When to implement (MVP gate)

Implement only when **all** hold:

1. Pure path stays zero-Rust; module ≤256 lines; fail-closed on unsupported surfaces  
2. Scope is **explicit**: e.g. pure `Int` params + `assert_eq!` expected values only  
3. Pass **and** fail rails (seeded domain that finds a known bug; rejects non-pure targets)  
4. No shell/eval of user fixtures beyond the existing Backend-C build pipeline  
5. Document residual (strings, caps, contracts still out)

## Interim agent guidance

- Use `ooda test <file.oo>` for verify/`assert_eq!`  
- Use corpus + `ooda check` for negative type/cap rails  
- Do not treat exit 2 on `--fuzz` as a product bug — it is intentional honesty  

## Related

- `bootstrap/BUILD_OUT.md` — P1 item  
- `scripts/ooda_test_verify.sh` — real test path  
- DESIGN § self-testing / fuzz pillars (north star; not auto-promoted to beta)
