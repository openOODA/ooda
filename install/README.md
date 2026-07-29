# openOODA install system

## Source of truth

| Artifact | Role |
|---|---|
| **`install/install.oo`** | Full installer — story, XDG layout, download, place, config, verify |
| **Website `install` / `install.sh`** | Chapter 0 bootstrap only (fetch stage-0, then `exec ooda run install.oo`) |
| **`scripts/release.sh`** | Packages `bin/ooda` + `install/install.oo` + `share/` + `std/` slot |

Shell cannot be eliminated for a **first** install (no `ooda` yet). Everything after that is OODA.

## XDG layout (after install.oo)

| Variable | Default | Contents |
|---|---|---|
| `OODA_HOME` | `$XDG_DATA_HOME/ooda` → `~/.local/share/ooda` | data: `std/`, `share/`, `install/` |
| `OODA_BIN` | `$XDG_BIN_HOME` → `~/.local/bin` | `ooda` binary on PATH |
| `OODA_CONFIG` | `$XDG_CONFIG_HOME/ooda` → `~/.config/ooda` | `env` shell snippet |
| `OODA_CACHE` | `$XDG_CACHE_HOME/ooda` → `~/.cache/ooda` | downloads + extract |
| `OODA_STD` | `$OODA_HOME/std` | standard library root (clone here) |
| `OODA_VERSION` | pin e.g. `v0.49.0-alpha` | release tag to fetch |

## User commands

```bash
# First time (bootstrap → install.oo)
curl -fsSL https://openOODA.github.io/install | sh

# Re-run story installer (already have ooda)
. ~/.config/ooda/env
ooda run ~/.local/share/ooda/install/install.oo
# or from a checkout:
ooda run install/install.oo
```

## Release pack

```bash
./scripts/release.sh          # version from Cargo.toml
./scripts/release.sh v0.49.0-alpha
```
