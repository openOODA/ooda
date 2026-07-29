## v0.34.0-alpha — Patch surface, WASM contract refuse, must-use honesty

### Top-5 this rotation

1. **Ship honesty:** extended `ooda patch` (params / return type / requires / ensures) from HEAD is **versioned** — no longer unreleased on the previous pin.
2. **Contracts:** `build --target wasm` (and llvm) refuse `requires`/`ensures` like C/native — no silent contract strip.
3. **Types:** `let _ = result` is must-use fail-closed (aligned with error text).
4. **Dual engine:** CHS parity digests include strings/bools (`chs_list_string` → `…,f,n,fn,true`).
5. **Caps / docs / E-M:** std net/fs pass cap tokens into sealed calls; nested docs updated from stale v0.9 lies; `examples/em_demo.oo` for measured `ooda em`.

### Pin
v0.34.0-alpha — Cargo, clap, BOOTSTRAP_PIN, install.oo, website install, docs brand.

### Not claimed
True object-cap (ambient grant still exists for free sealed ops), full native contract lowering, zero-`.rs` beta, full WASM product.
