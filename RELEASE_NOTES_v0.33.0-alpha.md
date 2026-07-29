## v0.33.0-alpha — Monorepo paths, CHS parity honesty, contracts refuse C

### Top-5 this rotation

1. **QA monorepo paths:** resolve `../ooda` (Projects/openOODA layout); parity script hard-fails if missing.
2. **Types:** if/else concrete type mismatch fails closed (Void arms still ok for stmt nesting).
3. **Dual engine:** `chs_hello.oo` restored; parity lists while + list/string + hello; **FAIL if listed file missing**; uses `chs_list_string`.
4. **Contracts honesty:** `build --target c|native|chs` **refuses** programs with `requires`/`ensures` (use `ooda run`/`test`).
5. **AI diagnostics:** TypeError `suggested_fix` names undefined function / unknown method; portability: C runtime/libooda paths via `CARGO_MANIFEST_DIR`.

### Pin
v0.33.0-alpha — Cargo, clap, BOOTSTRAP_PIN, install.oo, website install.

### Not claimed
Full native contract lowering, zero `.rs` beta, LSP, full LLVM List/String.
