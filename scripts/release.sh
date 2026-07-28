#!/usr/bin/env bash
# ===================================================================
# openOODA GitHub Release Packaging & Binary Publisher
# Builds a tarball under dist/ (gitignored) and uploads to GitHub Releases.
# Does NOT commit release artifacts into the git tree.
# ===================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-v0.12.1-alpha}"
# Allow VERSION with or without leading v
case "$VERSION" in
  v*) TAG="$VERSION" ;;
  *) TAG="v${VERSION}" ;;
esac
ARCH="linux-x86_64"
DIST_DIR="$ROOT/dist/ooda-${TAG}-${ARCH}"
TARBALL="$ROOT/dist/ooda-${TAG}-${ARCH}.tar.gz"

echo "[openOODA Release] Building ${TAG}..."
cargo build --release

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp target/release/ooda "$DIST_DIR/"
cp README.md DESIGN.md "$DIST_DIR/" 2>/dev/null || cp README.md "$DIST_DIR/"

tar -czf "$TARBALL" -C "$ROOT/dist" "ooda-${TAG}-${ARCH}"
echo "[openOODA Release] Archive: $TARBALL"

if gh release view "$TAG" --repo openOODA/ooda >/dev/null 2>&1; then
  echo "[openOODA Release] Uploading asset to existing release ${TAG}..."
  gh release upload "$TAG" "$TARBALL" --repo openOODA/ooda --clobber
else
  echo "[openOODA Release] Creating release ${TAG}..."
  gh release create "$TAG" "$TARBALL" \
    --repo openOODA/ooda \
    --title "openOODA ${TAG}" \
    --notes "Pre-built Linux x86_64 binary for the OODA toolchain (${TAG}). Source: https://github.com/openOODA/ooda"
fi

echo "[openOODA Release] Done. Artifacts remain under dist/ (gitignored)."
