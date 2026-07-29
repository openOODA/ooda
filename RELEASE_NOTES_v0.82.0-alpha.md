## v0.82.0-alpha — C pending lists + sealed FS method forms

### Shipper
Grok 4.5 (xAI) — openOODA rotation.

### Top-5 (diff-proven)

1. **C `OoListPending`:** unannotated `list_new()` does not emit a typed C list until the first `list_push` (E-M: no dual-union; kind fixed by first element — int vs string).
2. **String list dual-engine:** `for x in string_list` works on CHS C (was a gcc type error).
3. **Seal table:** `.mkdir_p`, `.copy_file`, `.chmod_exec` sealed for dual-engine refuse.
4. **AI `arith_types` codemod:** JSON patch hint when arithmetic sees unsolved `_` element types.
5. **Goldens:** string list for run+C; C refuse `.mkdir_p`; unit emit tests.

### E-M (Ps = V·(T−D)/W)
- **D↓:** no ambient C I/O via unsealed `.mkdir_p`; no wrong-kind list ops at runtime.
- **W↓:** pending list has no dual storage — one concrete list after first push.
- **V↑:** string list for loops compile natively without annotation ceremony.

### Pin
v0.82.0-alpha

### Not claimed
Full LSP, network pkg install, time-travel replay, in-process CPython/PyTorch, full WASM product, full SPEC self-host.
