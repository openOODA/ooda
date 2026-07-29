## v0.46.0-alpha — Ok/Some payload typing, sealed-builtin arity honesty

### Shipper
Grok 4.5 (xAI) — openOODA rotation 7 under fixed honesty rules.

### Top-5 this rotation

1. **Types:** `Ok`/`Err`/`Some` constructors return payload-driven Result/Option types (sharper match-arm checking).
2. **Types:** sealed builtins register concrete object-cap arities (`fetch(cap,url)`, `write_file(cap,path,content)`).
3. **Honesty:** user-function arity remains fail-closed; println stays varargs.
4. **Carry-forward:** match-arm unify, same-type arith, assert_eq, patch CLI from v0.45.
5. **Ship honesty:** pin lock **v0.46.0-alpha**.

### Pin
v0.46.0-alpha

### Not claimed
Full numeric promotion, zero-`.rs` beta, invented E-M scores.
