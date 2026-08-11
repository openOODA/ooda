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
| `byte.oo` | docs-only `type Byte = Int` + `byte_clamp` / `byte_in_range` (0..255 convention); check floor only | none |
| `markup/toml.oo` | path A single-line `key = value` / quoted string → `key\tvalue` | none |
| `markup/yaml.oo` | path A single-line `key: bare` → `key\tvalue` | none |
| `markup/xml.oo` | path A tag-strip text extract (no attributes) | none |
| `markup/json_schema.oo` | trivial `{}` schema id only; `validate` always false | none |
| `archive/{tar,zip,gzip}.oo` | path A magic detect only (not decompress) | none |

**Fixtures:**

- Check: `oodac check std/result.oo` (and `str.oo`, `option.oo`, `byte.oo`) — library OK without `main`.
- Build: `fixtures/std_result_main.oo`, `fixtures/std_str_main.oo` via
  `scripts/oodac_pure_build.sh` (multi-module emit + `chs_rt` only).
- Markup: `fixtures/std_markup_main.oo` + `scripts/std_markup_smoke.sh`
- Archive: `fixtures/std_archive_main.oo` + `scripts/std_archive_smoke.sh`

## Residual / path A seals (honest)

| Module | Role | Caps |
|--------|------|------|
| `os/sync.oo` | ThreadCap wrappers: `sync_mutex_lock` / `sync_mutex_unlock` / `sync_thread_spawn` | residual **Err** (no OS pthreads) |
| `os/thread.oo` | ThreadCap wrappers: `thr_mutex_*` / `thr_spawn` | residual **Err** |
| `os/gpu.oo` | GpuCap wrapper: `gpu_launch_shader` | residual **Err** (no shaders) |

| Item | Honesty |
|------|---------|
| `Option[T]` / `Some` / `None` as sum types | Typecheck may accept; **Backend-C does not lower** constructors → use `option.oo` Result encoding |
| Generic `Result[T,E]` beyond String | Runtime `OoResS` is string payload only on pure path |
| Native `&str` borrow + real `Byte` arrays | **Residual** — see `bootstrap/BYTE_STR.md`; `byte.oo` is Int convention only; `String` remains value-copy |
| `std::fs` / `std::net` / json / crypto (org sibling) | Cap-gated or host-era; **not** imported by pure std here |
| Thread/mutex/gpu free names | Path A seal only — granted cap returns residual Err, not real concurrency/GPU |
| Full XML/YAML/TOML/JSON Schema | Path A subsets only; else `UNIMPLEMENTED_RESIDUAL` — not DESIGN parsers |
| Full tar/zip/gzip decompress | Magic detect only; decompress NOT product |
| Multi-file `oodac check` of importers | Check is single-file; import resolution is pure-build residual (bash multi-emit) |

## Security

These modules must **not** open files, sockets, or env. They wrap only
pure CHS string/result surface already sealed in `runtime/chs_rt*.c`.
