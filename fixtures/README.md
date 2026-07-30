# fixtures/

Harness inputs for **tests**, **CHS parity**, and **fixed-point** scripts.

This is **not** a public tutorial pack. There is no `examples/` tree in the product
surface; historical demos remain recoverable from git history
(`git log -- examples/`, `git show <rev>:examples/hello.oo`).

| File | Used by |
|------|---------|
| `hello.oo` | golden tests, parity |
| `while_count.oo` | parity, build smoke |
| `int_main.oo` | host_api, fixed-point, parity |
| `chs_list_string.oo` | fixed-point, host_api, semantic parity |
| `chs_hello.oo` | semantic parity |
| `chs_fs_roundtrip.oo` | dual-engine refuse goldens |
| `unauthorized_io.oo` | capability JSON golden |
| `em_demo.oo` | `ooda em` measured report golden |
| `list_eq.oo` / `list_sum.oo` | WASM List[Int] host e2e |
| `string_ops.oo` / `string_walk.oo` | WASM string methods host e2e |
| `break_loop.oo` | while break/continue (interp + C + WASM + LLVM) |
| `for_range.oo` | for lo..hi desugar (interp + C + WASM + LLVM) |
| `str_concat.oo` | String + WASM bump-heap concat host e2e |
