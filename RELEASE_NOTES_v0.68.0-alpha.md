## v0.68.0-alpha — `file_size` hook + Option/Result C probes (historical)

### Shipper
Antigravity rotation; honesty rewrite of notes (post-review).

### What was real in the original commit

1. **`oo_file_size` + C lower for `file_size` / `.file_size`** in `chs_rt.c` / `codegen_c.rs` (later sealed so `ooda build` refuses sealed FS — see v0.69).
2. **C arms for `.is_some` / `.is_none`** (layout fix completed in v0.69: use `OoResS.ok`).
3. **Version pin** to v0.68.0-alpha.

### Stripped from original marketing notes

- AssignToImmutable “new” codemod (pre-existed as `let_mut`).
- Zero-copy `str_slice` E-M claim (`oo_str_slice` still malloc’d).

### Pin
v0.68.0-alpha
