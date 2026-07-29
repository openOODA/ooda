#!/usr/bin/env bash
# ===================================================================
# openOODA release packager
# Builds a user-facing tarball:
#   ooda-<tag>-linux-x86_64/
#     bin/ooda
#     install/install.oo      # story installer (source of truth)
#     share/VERSION
#     share/README.md
#     share/DESIGN.md         # if present
#     std/.gitkeep            # slot for OODA_STD (not a full std clone)
#
# Release train (do not reverse):
#   1) Bump + test ooda; notes must match `git show` for the tag
#   2) Tag ooda and publish the GitHub Release + tarball asset
#   3) Only then pin docs / openOODA.github.io / qa to the same version
# Sibling install scripts must not advertise a pin without a released asset.
# ===================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Version from Cargo.toml unless overridden
CARGO_VER="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
VERSION="${1:-v${CARGO_VER}}"
case "$VERSION" in
  v*) TAG="$VERSION" ;;
  *) TAG="v${VERSION}" ;;
esac

ARCH="linux-x86_64"
NAME="ooda-${TAG}-${ARCH}"
DIST_DIR="$ROOT/dist/${NAME}"
TARBALL="$ROOT/dist/${NAME}.tar.gz"

echo "[openOODA Release] Building ${TAG} from Cargo ${CARGO_VER}…"
cargo build --release

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/bin" "$DIST_DIR/install" "$DIST_DIR/share" "$DIST_DIR/std"

cp target/release/ooda "$DIST_DIR/bin/ooda"
chmod +x "$DIST_DIR/bin/ooda"

# Installer (OODA source of truth)
if [[ ! -f install/install.oo ]]; then
  echo "error: install/install.oo missing" >&2
  exit 1
fi
cp install/install.oo "$DIST_DIR/install/install.oo"

# Metadata
echo "$TAG" > "$DIST_DIR/share/VERSION"
cp README.md "$DIST_DIR/share/README.md"
if [[ -f LICENSE ]]; then
  cp LICENSE "$DIST_DIR/share/LICENSE"
fi
if [[ -f DESIGN.md ]]; then
  cp DESIGN.md "$DIST_DIR/share/DESIGN.md"
fi

# Std slot — empty marker (users clone openOODA/std here or set OODA_STD)
cat > "$DIST_DIR/std/README.md" <<EOF
# openOODA standard library slot

This directory is the default \`OODA_STD\` after install.

Clone the std library here:

  git clone https://github.com/openOODA/std.git .

Or point OODA_STD at any checkout:

  export OODA_STD=/path/to/openooda-std
EOF

# Self-describing layout note
cat > "$DIST_DIR/README.md" <<EOF
# openOODA ${TAG} (${ARCH})

## Layout
- \`bin/ooda\` — stage-0 toolchain
- \`install/install.oo\` — full installer (run with ooda)
- \`share/\` — VERSION + docs
- \`std/\` — default standard library root (empty until you clone)

## Install (recommended)
From this extracted tree (or after bootstrap has placed \`ooda\` on PATH):

  ./bin/ooda run install/install.oo

The installer writes an XDG-correct tree under \`~/.local/share/ooda\`,
puts the binary in \`~/.local/bin\`, and writes \`~/.config/ooda/env\`.

## Manual
  cp bin/ooda ~/.local/bin/
  export PATH="\$HOME/.local/bin:\$PATH"
EOF

mkdir -p "$ROOT/dist"
tar -czf "$TARBALL" -C "$ROOT/dist" "$NAME"
echo "[openOODA Release] Archive: $TARBALL"
echo "[openOODA Release] Contents:"
tar -tzf "$TARBALL" | head -30

if command -v gh >/dev/null 2>&1; then
  if gh release view "$TAG" --repo openOODA/ooda >/dev/null 2>&1; then
    echo "[openOODA Release] Uploading to existing ${TAG}…"
    gh release upload "$TAG" "$TARBALL" --repo openOODA/ooda --clobber
    # Also upload bare binary for people who only want the bit
    gh release upload "$TAG" "$DIST_DIR/bin/ooda" --repo openOODA/ooda --clobber 2>/dev/null || true
  else
    echo "[openOODA Release] Creating ${TAG}…"
    gh release create "$TAG" "$TARBALL" \
      --repo openOODA/ooda \
      --title "openOODA ${TAG}" \
      --notes-file "$DIST_DIR/README.md"
  fi
else
  echo "[openOODA Release] gh not available; tarball left at $TARBALL"
fi

echo "[openOODA Release] Done."
