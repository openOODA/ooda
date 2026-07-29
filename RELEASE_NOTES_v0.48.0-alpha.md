## v0.48.0-alpha — Missing returns, Int≠Float eq, if-value requires else

### Shipper
Grok 4.5 (xAI) — openOODA rotation 9 under fixed honesty rules.

### Top-5 this rotation (DESIGN pillars: types / contracts)

1. **Missing return fail-closed:** functions declared `-> T` (T≠Void) with Void body error (was green then printed `()`).
2. **Same-type equality:** `Int == Float` rejected at typecheck (was soft Bool / runtime false).
3. **If-as-value:** non-Void `if` without `else` rejected (false path was silent Void).
4. **Statement if** without else still accepted (Void then-branch).
5. **Ship honesty:** pin lock **v0.48.0-alpha** + unit tests for all four behaviors.

### Pin
v0.48.0-alpha — Cargo, clap, CANONICAL, BOOTSTRAP_PIN, install.oo, README, docs, QA, website install.

### Not claimed
Full control-flow return analysis (only end-of-body Void check), zero-`.rs` beta, invented E-M scores.
