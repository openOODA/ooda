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
