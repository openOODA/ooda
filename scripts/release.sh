#!/usr/bin/env bash
# ===================================================================
# openOODA GitHub Release Packaging & Binary Publisher Script
# ===================================================================
set -e

VERSION="v0.1.1-alpha"
ARCH="linux-x86_64"
DIST_DIR="dist"
TARBALL="ooda-${VERSION}-${ARCH}.tar.gz"

echo "🔨 [openOODA Release Publisher] Building native binary release ${VERSION}..."
cargo build --release

mkdir -p "$DIST_DIR"
cp target/release/ooda "$DIST_DIR/"
cp README.md "$DIST_DIR/"
cp DESIGN.md "$DIST_DIR/"

tar -czvf "$TARBALL" -C "$DIST_DIR" .

echo "📦 Created release binary archive: ${TARBALL}"

if gh release view "$VERSION" > /dev/null 2>&1; then
    echo "Updating existing release ${VERSION} on GitHub..."
    gh release upload "$VERSION" "$TARBALL" --clobber
else
    echo "Creating new release ${VERSION} on GitHub..."
    gh release create "$VERSION" "$TARBALL" \
        --title "openOODA ${VERSION} Compiler Release" \
        --notes "Official pre-built binary release for the OODA programming language compiler toolchain."
fi

echo "🚀 Successfully published GitHub Release ${VERSION} with native binary assets!"
