## v0.9.0-alpha — Runtime Capability Gate + Real async/thread + Honest stdlib

### Round-2 Highlights (this push)

1. **Runtime capability enforcement** — `Interpreter::call_function` now
   consults the sealed `EFFECT_BUILTINS` table and refuses to invoke a
   sealed effect (fetch, .read_file, async_spawn_internal,
   python_embed_internal, ...) unless the enclosing function declared the
   matching cap token in its parameter list. This is enforced even if the
   static `CapabilityChecker` is bypassed. Covered by three unit tests.

2. **Real `std::thread` async** — `async_spawn_internal` spawns a real OS
   thread via `std::thread::Builder::spawn` and returns a numeric handle
   `"thread#N"`. `async_join_internal` joins the thread by id and returns
   its result. No more `format!`-built fake handles.

3. **Honest `python_embed_internal`** — returns a clear
   `Err("not implemented in this alpha")` instead of a fake handle string.
   Callers can no longer believe a model was loaded.

4. **Sealed effect table extended** — `async_spawn_internal`,
   `async_join_internal`, and `python_embed_internal` were added to
   `EFFECT_BUILTINS`, each requiring `&SysCap`. Both the static and runtime
   gates protect them.

5. **Dist binary refreshed** — `dist/ooda` and
   `ooda-v0.9.0-alpha-linux-x86_64.tar.gz` rebuilt against this commit.

### Earlier in v0.9.0-alpha (kept from previous round)

- Static type checker runs on `run` / `test` / `build` before execution.
- Rejects undefined variables, bad arithmetic (e.g. `true + 1`), non-Bool
  `if` conditions.
- Division by zero returns a runtime language error (no host panic).
- Improved contract fuzzer: multi-param matrix, precondition rejects counted
  separately.
- Real SHA-256 / HMAC-SHA256 via the `sha2` and `hmac` crates (verified
  against RFC test vectors in the QA suite).
- Real JSON parse / stringify via `serde_json`.
- LLVM IR integer-subset backend validates its own output (duplicate
  ret, undefined attribute groups, type mismatches).

### Install

```bash
tar xzf ooda-v0.9.0-alpha-linux-x86_64.tar.gz
export PATH="$PWD/ooda-v0.9.0-alpha-linux-x86_64:$PATH"
ooda --version   # 0.9.0-alpha
ooda run examples/hello.oo
ooda test examples/math_contract.oo
```

### Honest gaps in this alpha

- **WASM target** not implemented (`--target wasm` is rejected).
- **`lsp`, `outline`, `patch`, `context`, `replay`, `migrate`, `pkg`** are
  scaffolding and print placeholder output.
- **Python / HuggingFace / PyTorch bridge** — `python_embed_internal`
  returns an honest `Err`.
- **No "0ms GC pauses"** — the interpreter uses `Box` / `HashMap`. There
  is no GC and no region arena.