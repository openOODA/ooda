# openOODA v0.115.0-alpha
Shipper: **Grok 4.5 (xAI)** — R1 typecheck slice in oodac (.oo).

## Top-5 (first principles; zero-Rust is a peer pillar)
1. **Honesty D↓:** oodac `check` used to print `OK` on programs stage-0 type-rejects (`let x: Int = "hi"`).
2. **R1 typecheck slice in `.oo`:** annotated `let` + `return` literals → `ERR\ttype\t…` + non-zero exit.
3. **No short-circuit footgun:** sequential bounds guards (OODA `&&` evaluates both sides).
4. **Corpus:** `bootstrap/corpus/typecheck/{pass,fail}/*.oo`.
5. **Parity:** `scripts/chs_parity.sh` R1 typecheck section + cargo test.

## E-M
- **D → 0:** silent green typecheck was pure drag on the self-host path.
- **W ↓:** token walk only (no heap ASTs beyond existing lists); stack locals for scan state.
- **V ↑:** real `.oo` type gate advances beta without rewriting stage-0 Rust yet.

## Scoreboard
- **RS_COUNT:** still stage-0 present (no `.rs` deleted this cycle — slice only).
- **Not beta:** full typecheck/eval self-host still open.

## Pin triple
**v0.115.0-alpha** — tag / Release / BOOTSTRAP_PIN / site install.
