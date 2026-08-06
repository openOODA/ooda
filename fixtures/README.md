# fixtures/

Harness inputs for **tests**, **CHS parity**, **fixed-point**, and product smokes.

This is **not** a public tutorial pack. Prefer `bootstrap/corpus/` for pass/fail rails.

## Pure product path (v0.182.1-alpha)

| File | Used by |
|------|---------|
| `hello.oo` / `int_main.oo` / `while_count.oo` | parity, build, contracts |
| `chs_list_string.oo` / `chs_hello.oo` | fixed-point, semantic parity |
| `chs_fs_roundtrip.oo` | FsCap write/read (no match; pure Backend-C) |
| `verify_pass.oo` / `verify_fail.oo` | `ooda test` rails |
| `outline_reflect_pass.oo` | outline/reflect smoke |
| `patch_add.oo` (+ body txt) | patch smoke |
| `std_{result,str,option}_main.oo` | `scripts/std_smoke.sh` |
| `unauthorized_io.oo` | cap deny (manual / residual) |

## Residual / historical (not pure-path claims)

| File | Note |
|------|------|
| `list_*.oo`, `string_*.oo`, `break_loop.oo` | WASM/host-era e2e; not Backend-C product claims |
| `for_range.oo` (+ `.wat`) | **Residual:** range-for not lowered on Backend-C (`ERR\tc_emit\tfor residual`). Historical WASM/host only — see `bootstrap/FOR_MATCH_RESIDUAL.md` |
| `*.wat` | **Archived** WASM text modules from host-era dual-engine; not built/run by pure product rails |
| `em_demo.oo` | historical `ooda em` (command residual) |

Keep `*.wat` (+ matching historical `.oo`) for archaeology only — do not reintroduce a WASM product path.

**Product truth:** `ooda run` = native Backend-C build+exec. No interpreter / LLVM / WASM product path.
Use `while` (not `for i in a..b`) and `if is_ok` (not stmt `match`) on the pure emit-c path.
