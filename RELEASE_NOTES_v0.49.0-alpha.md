## v0.49.0-alpha — Always-returns analysis, no Void binds, const /0

### Shipper
Grok 4.5 (xAI) — openOODA rotation 10 under fixed honesty rules.

### Top-5 this rotation (types / contracts / E-M D↓)

1. **Always-returns analysis:** `if/else` both `return` paths accepted for non-Void fns (fixes v0.48 false-positive missing-return).
2. **Partial return fallthrough** still fails (`if cond { return x; }` without else).
3. **No Void let-binds:** `let x = while …` rejected (was green then printed `()`).
4. **Const division by zero:** `1/0` and `1.0/0.0` fail at typecheck (was runtime trap).
5. **Ship honesty:** pin lock **v0.49.0-alpha** + unit tests for all five behaviors.

### Pin
v0.49.0-alpha

### Not claimed
Full CFG reachability, loop invariant returns, zero-`.rs` beta, invented E-M scores.
