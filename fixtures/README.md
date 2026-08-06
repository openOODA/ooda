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
| `list_*.oo`, `string_*.oo`, `for_range.oo`, `break_loop.oo` | WASM/host-era e2e; not Backend-C product claims |
| `em_demo.oo` | historical `ooda em` (command residual) |

**Product truth:** `ooda run` = native Backend-C build+exec. No interpreter / LLVM / WASM product path.
