# fixtures/

Harness inputs for **tests**, **CHS parity**, **fixed-point**, and product smokes.

This is **not** a public tutorial pack. Prefer `bootstrap/corpus/` for pass/fail rails.

## Pure product path (v0.184.0-alpha+)

| File | Used by |
|------|---------|
| `hello.oo` / `int_main.oo` / `while_count.oo` | parity, build, contracts |
| `chs_list_string.oo` / `chs_hello.oo` | fixed-point, semantic parity |
| `chs_fs_roundtrip.oo` | FsCap write/read (runtime + static seal) |
| `verify_pass.oo` / `verify_fail.oo` | `ooda test` rails |
| `outline_reflect_pass.oo` | outline/reflect smoke |
| `patch_add.oo` (+ body txt) | patch smoke |
| `std_{result,str,option}_main.oo` | `scripts/std_smoke.sh` |
| `result_unwrap.oo` | Result `.unwrap()` Ok path |
| `result_unwrap_err.oo` | Result `.unwrap()` Err → `ERR\tunwrap` + exit 1 |
| `ensures_simple.oo` / `ensures_fail.oo` | ensures runtime rails |
| `unauthorized_io.oo` | static cap deny (check) |
| `em_demo.oo` | measured `ooda em` (In — parse/cap/typecheck µs + weight; no fake drag-%) |

Corpus (primary rails):

| Path | What |
|------|------|
| `bootstrap/corpus/emit-c/pass/match_result_stmt.oo` | stmt match Result |
| `bootstrap/corpus/emit-c/pass/match_result_let.oo` | match-let Result |
| `bootstrap/corpus/emit-c/pass/result_unwrap_ok.oo` | unwrap Ok |
| `bootstrap/corpus/emit-c/pass/cap_runtime_read.oo` | runtime cap + read |
| `bootstrap/corpus/emit-c/fail/match_incomplete.oo` | incomplete match fail-closed |
| `bootstrap/corpus/emit-c/pass/for_range_int.oo` | INT..INT range-for |

## Residual / historical (not pure-path claims)

| File | Note |
|------|------|
| `list_*.oo`, `string_*.oo`, `break_loop.oo` | WASM/host-era e2e; not Backend-C product claims |
| `for_range.oo` (+ `.wat`) | Demo only; product lowers **INT..INT** only — non-INT → residual (`FOR_MATCH_RESIDUAL.md`) |
| `*.wat` | **Archived** WASM text modules from host-era dual-engine; not built/run by pure product rails |

Keep `*.wat` (+ matching historical `.oo`) for archaeology only — do not reintroduce a WASM product path.

**Product truth:** `ooda run` = native Backend-C build+exec. No interpreter / LLVM / WASM product path.  
**Match / unwrap / for:** Result `match` stmt + match-let + `.unwrap()` + `for i in INT..INT` are lowered — see `bootstrap/FOR_MATCH_RESIDUAL.md`.  
**Caps:** static check + runtime magic-token seal — see `bootstrap/STATIC_CAPS.md`.
