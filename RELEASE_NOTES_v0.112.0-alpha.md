# openOODA v0.112.0-alpha
Shipper: **Grok 4.5 (xAI)** — dual-engine honesty + string concat cycle.

## Top-5 (highest leverage)
1. **WASM while-body tail honesty** — idiomatic `if cond { break; }` without trailing `;` is `body.expr`; no longer silently dropped (was miscompiling control flow).
2. **LLVM while + break/continue** — loop stack labels; tails and nested ifs lower; stack `alloca` for scalars (W↓).
3. **LLVM if side-effects** — `println` / assign / break in branches no longer silently discarded (prior alpha only lowered `return`).
4. **WASM String `+` concat** — pure-WAT bump-heap copy; no host strcat; heap gated on real use (W↓).
5. **Fixtures + host e2e** — `break_loop.oo`, `for_range.oo`, `str_concat.oo` with wasmtime host proofs.

## E-M (Ps = V · (T − D) / W)
- **D → 0**: silent dual-engine drops of break/if were architectural drag (wrong IR looked green). Fail-honest lowering restores thrust.
- **W ↓**: heap/list RT still gated; string concat reuses fixed scratch locals (no per-call heap for labels); LLVM locals stay stack `alloca`.
- **V ↑**: same Int programs now correct across interp / C / WASM / LLVM without rework loops from wrong codegen.

## Not claimed
Full WASM product (List[String]/caps/struct/match still refuse), full LSP, zero-`.rs` beta, package registry.

## Pin triple
git tag / GitHub Release / `install/BOOTSTRAP_PIN` + website install → **v0.112.0-alpha**

RS_COUNT (stage-0) remaining toward beta: report via `find . -name '*.rs' -not -path './.git/*' -not -path './target/*' | wc -l`
