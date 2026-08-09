# openOODA install system

## Source of truth

| Artifact | Role |
|---|---|
| **`install/install.oo`** | Full installer — story, XDG layout, download, place, config, verify |
| **Website `install` / `install.sh`** | Chapter 0 bootstrap only (fetch prebuilt binary, then hand off) |
| **`scripts/release.sh`** | Packages pure `bin/ooda` + `oodac` + `install/install.oo` + `share/` + runtime C; writes `.sha256` |
| **`scripts/bootstrap_no_cargo.sh`** | Rebuild pure product from seed + gcc |
| **`scripts/install_dress_rehearsal.sh`** | Offline layout dress rehearsal of release tarball / staged tree |
| **`bootstrap/RELEASE_CHECKLIST.md`** | Pin lock + release notes habit (not beta gate) |

Shell cannot be eliminated for a **first** install (no `ooda` yet). Everything after that is OODA.

## XDG layout (after install.oo)

| Variable | Default | Contents |
|---|---|---|
| `OODA_HOME` | `$XDG_DATA_HOME/ooda` → `~/.local/share/ooda` | data: `std/`, `share/`, `install/` |
| `OODA_BIN` | `$XDG_BIN_HOME` → `~/.local/bin` | `ooda` binary on PATH |
| `OODA_CONFIG` | `$XDG_CONFIG_HOME/ooda` → `~/.config/ooda` | `env` shell snippet |
| `OODA_CACHE` | `$XDG_CACHE_HOME/ooda` → `~/.cache/ooda` | downloads + extract |
| `OODA_STD` | `$OODA_HOME/std` | standard library root (clone here) |
| `OODA_VERSION` | pin e.g. `v0.183.0-alpha` | release tag to fetch |

## User commands

```bash
# First time (bootstrap → install.oo)
curl -fsSL https://openOODA.github.io/install | sh

# From a pure checkout (seed + gcc)
export SEED_OODAC="${SEED_OODAC:-$PWD/oodac/oodac}"
./scripts/bootstrap_no_cargo.sh
./bin/ooda version
```

## Release pack

```bash
./scripts/release.sh                 # pin from install/BOOTSTRAP_PIN
./scripts/release.sh v0.183.0-alpha
```

Pure product pack: requires seed + gcc to rebuild if binaries missing.
