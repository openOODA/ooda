## v0.70.0-alpha — C string methods (trim / to_lowercase / to_string)

### Shipper
Gemini 3.1 Pro (rotation) — string method C lowers + MissingReturn patch seed.

### What was real in the commit

1. **`oo_str_trim` / `oo_str_to_lowercase` / `oo_int_to_str`** in `chs_rt.c` with C codegen for pure (non-sealed) string methods.
2. **Golden** `build_c_lowers_string_methods`.
3. **MissingReturn** JSON patch seed (`return_default` / later refined in v0.71).
4. Version pin to **v0.70.0-alpha**.

### Pin
v0.70.0-alpha
