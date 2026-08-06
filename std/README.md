# openOODA `ooda/std` — pure-path standard library

**Scope:** Modules that **check + Backend-C build** on the pure product path  
(`.oo` + thin C runtime). Capability-honest: **no ambient FS/net** here.

Sibling org repo `openOODA/std` (json/crypto/net/fs aspirational APIs) is
**not** the pure-path floor. This tree is what product programs can import today.

## Real (pure check + pure build)

| Module | Role | Caps |
|--------|------|------|
| `result.oo` | `Result[String,String]` helpers (`res_ok` / `res_err` / `res_unwrap_or` / …) | none |
| `str.oo` | thin string wrappers (`str_len`, `str_eq`, `str_contains`, `str_concat`, `str_sub`, …) | none |
| `option.oo` | optional-string helpers encoded as `Result` (`opt_some` / `opt_none` / …) | none |

**Fixtures:**

- Check: `oodac check std/result.oo` (and `str.oo`, `option.oo`) — library OK without `main`.
- Build: `fixtures/std_result_main.oo`, `fixtures/std_str_main.oo` via
  `scripts/oodac_pure_build.sh` (multi-module emit + `chs_rt` only).

## Residual (not pure-lowered / not claimed)

| Item | Honesty |
|------|---------|
| `Option[T]` / `Some` / `None` as sum types | Typecheck may accept; **Backend-C does not lower** constructors → use `option.oo` Result encoding |
| Generic `Result[T,E]` beyond String | Runtime `OoResS` is string payload only on pure path |
| `std::fs` / `std::net` / json / crypto (org sibling) | Cap-gated or host-era; **not** imported by pure std here |
| Multi-file `oodac check` of importers | Check is single-file; import resolution is pure-build residual (bash multi-emit) |

## Security

These modules must **not** open files, sockets, or env. They wrap only
pure CHS string/result surface already sealed in `runtime/chs_rt*.c`.
