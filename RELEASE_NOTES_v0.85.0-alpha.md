## v0.85.0-alpha — break/continue + honest pkg tarball install

### Shipper
Grok 4.5 (xAI) — openOODA rotation.

### Top-5 (diff-proven)

1. **`break` / `continue`:** statements in while/for (for desugars to while); typecheck loop-depth; interp + CHS C; LLVM/WASM refuse honestly.
2. **pkg install honesty:** remote only **https://…/*.tar.gz|*.tgz** via host curl+tar; git@ / *.git / non-tarball URLs fail non-zero; no registry/sigs.
3. **C `.push`:** method name aliases `list_push` for dual-engine list building.
4. **Seal `.env_set`:** method form sealed like free `env_set`.
5. **Goldens:** break/continue run+C; break outside loop; unsupported remote pkg URLs.

### E-M (Ps = V·(T−D)/W)
- **D↓:** no silent wrong-path package installs; no ambient loop control bugs; sealed env_set methods.
- **W↓:** break/continue are zero-cost control flags (no heap alloc).
- **V↑:** early-exit loops without nested flag variables; native C emits real `break`/`continue`.

### Pin
v0.85.0-alpha

### Not claimed
Full LSP, package registry, git clone, signed packages, time-travel replay, in-process CPython, full WASM, full self-host.
