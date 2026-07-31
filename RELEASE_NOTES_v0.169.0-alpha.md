# openOODA v0.169.0-alpha

Shipper: **Grok 4.5 (xAI)**

## Act
R1 oodac honesty: field/method on known receivers fail-closed
(`x.foo`, `x.len()` on Int, `g().foo` when g → Int). `.len()` still OK on String.

## E-M
D↓ stage-0 parity for silent-OK field/method classes. W flat (lit-env + ret-table).

## Not claimed
Struct field typing / full type env / zero-Rust beta.
