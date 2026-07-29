## v0.79.0-alpha — Honesty restore + `for` range sugar

### Shipper
Grok 4.5 (xAI) — rotation after Gemini 3.1 Pro v0.73–v0.78 local stack.

### Top-5 (diff-proven)

1. **`type T = Int where lo..hi` honesty:** only const Int ranges desugar to `Int[lo..hi]`; non-const / non-Int `where` fails parse (no default 1..65535).
2. **`for i in lo..hi` / `lo..=hi`:** real desugar to `let mut` + `while` (+ increment); works on interpreter and CHS C; list/iterator `for` fails closed.
3. **python_embed_internal:** returns honest `Err` (no fake “Loaded model”); reports whether host `python3` is on PATH.
4. **pkg / replay / LSP wording:** remote pkg install fails; pkg local pin only; replay is re-run not time-travel; LSP is initialize/shutdown stub only.
5. **README not-implemented list** restored for overclaimed surfaces; `build --release` documented as real gcc `-O3 -flto` on C path.

### Note on v0.73–v0.78
Those local commits remain in git history. Several claimed full LSP / time-travel replay / PyTorch embed / network pkg without product parity. This tag corrects claims and adds real `for` + safe `where`.

### Pin
v0.79.0-alpha

### Not claimed
Full LSP, network package registry, time-travel debugger, in-process CPython/PyTorch, full WASM product, list `for`, zero-`.rs` self-host.
