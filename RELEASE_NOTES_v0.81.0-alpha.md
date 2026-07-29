## v0.81.0-alpha — List-for element refinement + zero-warning stage-0

### Shipper
Grok 4.5 (xAI) — openOODA rotation.

### Top-5 (diff-proven)

1. **List element type refinement on assign:** `xs = list_push(xs, 10)` after `list_new()` refines `List[_]` → `List[Int]` so unannotated **`for x in xs`** typechecks.
2. **List-for dual-engine goldens:** `for x in list` run + CHS C (sum 10+20=30) without `List[Int]` annotation.
3. **Stage-0 warning-free:** removed unused import/`bind_pattern` wrapper; `AnalyzeTimings::attach` used for JSON diagnostics.
4. **Honesty:** v0.80 list-for desugar kept; README claims only what tests prove.
5. **Pin train** to **v0.81.0-alpha** (ooda → release → docs/site/qa).

### Pin
v0.81.0-alpha

### Not claimed
Full LSP, network pkg install, time-travel replay, in-process CPython/PyTorch, full WASM product, full SPEC self-host.
