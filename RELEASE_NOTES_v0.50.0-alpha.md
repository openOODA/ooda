## v0.50.0-alpha — Early return, unreachable after return, AI patches

### Shipper
Grok 4.5 (xAI) — openOODA rotation 11 under fixed honesty rules.

### Top-5 this rotation (types / contracts / E-M D↓)

1. **Early return always-returns:** a plain `return` (or if/else both returning) marks the function as returning — fixes false-positive “missing return” when dead code follows.
2. **Unreachable after return:** statements after a path that returned fail-closed with `unreachable code after return`.
3. **Partial if-return still fails:** `if cond { return x; }` without else remains missing-return (no theater CFG).
4. **AI `--json-errors` patches:** missing return, unreachable after return, and const division-by-zero get `applicability: true` patch hints.
5. **Ship honesty:** pin lock **v0.50.0-alpha** + unit + golden tests; E-M remains measured-only (no invented scores).

### Pin
v0.50.0-alpha

### Not claimed
Full CFG reachability, loop-exit returns, zero-`.rs` beta, invented E-M drag-% or Boyd Ps scores.
