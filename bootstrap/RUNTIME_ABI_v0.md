# Runtime ABI v0 (C realization) — measured from pure emit

**Backend:** C (`runtime/chs_rt*`, Backend-C emit/link).  
**How measured:** pure `oodac emit-c` on fixtures (2026-03 pin tree); symbols grepped from generated C.  
**Honesty:** preamble **declares** a wide symbol set; a given program may **call** only a subset. Host-era symbols still appear in decls even when body never calls them.

---

## Value types (`chs_rt.h` + emit preamble)

| ABI name | C shape |
|----------|---------|
| `OoStr` | `{ char *data; long long len; }` |
| `OoIList` | `{ long long *data; long long len; long long cap; }` |
| `OoSList` | `{ OoStr *data; long long len; long long cap; }` |
| `OoResS` | `{ int ok; OoStr val; }` |
| `OoResV` | `{ int ok; OoStr err; }` |

---

## Symbols declared by pure emit preamble (Backend-C)

### Defined in `runtime/chs_rt*.c` (link against chs_rt)

| Symbol | Family | TU (primary) |
|--------|--------|----------------|
| `oo_str_lit` | string | `chs_rt_str.c` |
| `oo_str_concat` | string | `chs_rt_str.c` |
| `oo_str_byte_len` | string | `chs_rt_str.c` |
| `oo_chars_len` | string | `chs_rt_str.c` |
| `oo_char_at` | string | `chs_rt_str.c` |
| `oo_str_slice` | string | `chs_rt_str.c` |
| `oo_str_contains` | string | `chs_rt_str.c` |
| `oo_str_eq` | string | `chs_rt_str.c` |
| `oo_str_trim` | string | `chs_rt_str.c` |
| `oo_str_to_lowercase` | string | `chs_rt_str.c` |
| `oo_int_to_str` | string | `chs_rt_str.c` |
| `oo_char_is_digit` | string | `chs_rt_str.c` |
| `oo_char_is_alpha` | string | `chs_rt_str.c` |
| `oo_char_is_space` | string | `chs_rt_str.c` |
| `oo_ilist_new` / `push` / `get` / `len` | list int | `chs_rt_list.c` |
| `oo_slist_new` / `push` / `get` / `len` | list str | `chs_rt_list.c` |
| `oo_print_str` / `oo_print_int` / `oo_print_bool` / `oo_println` | print | `chs_rt_print.c` |
| `oo_read_file` / `oo_write_file` | fs | `chs_rt_fs.c` |
| `oo_path_exists` / `oo_file_size` | fs | `chs_rt_fs.c` |
| `oo_env_get` | env | `chs_rt_fs.c` (or host split) |

### Host residual — **not** in pure emit preamble (removed)

Pure Backend-C preamble no longer declares host-era symbols. Optional only under
`OODA_WITH_HOST_FFI` in `runtime/chs_rt_host.c` (legacy):

| Symbol | Notes |
|--------|--------|
| `oo_host_ast_dump` / `oo_host_check` / `oo_host_token_dump` | wrappers → `ooda_host_*` |
| `oo_chs_build` | wrapper → `ooda_host_chs_build` |
| `ooda_host_ast_dump` / `check` / `token_dump` / `chs_build` / `free` | C host FFI names |

Pure default link (`chs_rt.c` without host FFI) does **not** define them; pure
product emit must not reference them. Residual: optional host FFI path only.
### Inline in emit preamble (not separate .c exports)

| Symbol | Notes |
|--------|--------|
| `oo_process_exit` | `static inline` → `exit` |
| `oo_sys_exec1` | `static inline` → `system` |

### Internal only (runtime, not for emit surface)

| Symbol | Notes |
|--------|--------|
| `oo_from_c_heap` | static helper in runtime |

---

## Symbols actually **called** by measured fixture bodies

### `fixtures/chs_list_string.oo` (body)

`oo_ilist_new`, `oo_ilist_push`, `oo_ilist_len`, `oo_ilist_get`,  
`oo_str_lit`, `oo_chars_len`, `oo_char_at`, `oo_str_slice`, `oo_char_is_alpha`,  
`oo_print_int`, `oo_print_str`, `oo_print_bool`, `oo_println`,  
`oo_slist_new`, `oo_slist_push` (argv args list in main inject).

### `fixtures/while_count.oo`

`oo_print_int`, `oo_println` (+ main args list inject).

### `fixtures/chs_hello.oo` / simple mains

Primarily print + `oo_str_lit` as needed.

**Fact:** pure preamble no longer declares host residual (`oo_host_*` / `ooda_host_*` /
`oo_chs_build`). It still declares the full string/fs set even when unused — floor
surface is wider than smoke call set (emit surface residual, not host landmine).

---

## Link recipe (Backend-C)

Single module:

```bash
gcc -O2 -Iruntime runtime/chs_rt.c <generated.c> -lm -o <out>
```

Multi-module: `scripts/oodac_pure_build.sh`.

---

## Anti-creep rules

1. New `oo_*` used by emit → row in this file same change.  
2. Prefer shrinking preamble decls over growing silent host residual.  
3. Second backend (FLOOR F3) must cover at least the **called** set for CHS smokes; full declared set can wait.

---

## Measurement command (re-run)

```bash
EMIT_NO_CONCAT=1 ./oodac/oodac emit-c fixtures/chs_list_string.oo | grep -oE 'oo_[A-Za-z0-9_]+' | sort -u
```

---

*v0 = measured + honest residual host decls. Bump v1 when a second backend forces a portable subset.*
