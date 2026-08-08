#!/usr/bin/env bash
# openOODA release packager — pure .oo + C path (no cargo/rustc)
# Builds: ooda-<tag>-linux-x86_64/{bin/ooda,oodac/oodac,install,share,runtime}
# Habit: bootstrap/RELEASE_CHECKLIST.md (pin → notes → rails → pack → dress)
# Does not force beta tag.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PIN_FILE="$ROOT/install/BOOTSTRAP_PIN"
if [[ -f "$PIN_FILE" ]]; then
  CARGO_VER="$(tr -d 'v\r\n' <"$PIN_FILE" | head -1)"
else
  CARGO_VER="0.183.0-alpha"
fi
VERSION="${1:-v${CARGO_VER}}"
case "$VERSION" in
  v*) TAG="$VERSION" ;;
  *) TAG="v${VERSION}" ;;
esac

ARCH="linux-x86_64"
NAME="ooda-${TAG}-${ARCH}"
DIST_DIR="$ROOT/dist/${NAME}"
TARBALL="$ROOT/dist/${NAME}.tar.gz"
SHA_FILE="${TARBALL}.sha256"
NOTES="$ROOT/RELEASE_NOTES_${TAG}.md"
# filename uses tag without forcing v-prefix mismatch: RELEASE_NOTES_vX…
NOTES_ALT="$ROOT/RELEASE_NOTES_${TAG#v}.md"

echo "[openOODA Release] Building ${TAG} without cargo…"
if [[ ! -f "$NOTES" && ! -f "$NOTES_ALT" ]]; then
  echo "NOTE: missing RELEASE_NOTES for ${TAG} — see bootstrap/RELEASE_CHECKLIST.md" >&2
fi

# Ensure pure product binaries
if [[ ! -x "$ROOT/bin/ooda" ]] || [[ ! -x "$ROOT/oodac/oodac" ]]; then
  SEED_OODAC="${SEED_OODAC:-$ROOT/oodac/oodac}" "$ROOT/scripts/bootstrap_no_cargo.sh"
fi
if [[ ! -x "$ROOT/bin/ooda" ]]; then
  echo "error: bin/ooda missing after bootstrap" >&2
  exit 1
fi

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/bin" "$DIST_DIR/oodac" "$DIST_DIR/install" \
  "$DIST_DIR/share" "$DIST_DIR/std" "$DIST_DIR/runtime" "$DIST_DIR/scripts" "$DIST_DIR/cli"

cp "$ROOT/bin/ooda" "$DIST_DIR/bin/ooda"
chmod +x "$DIST_DIR/bin/ooda"
cp "$ROOT/oodac/oodac" "$DIST_DIR/oodac/oodac"
chmod +x "$DIST_DIR/oodac/oodac"

# Seed sources for rebuild (pure self-host)
cp -a "$ROOT/oodac/"*.oo "$DIST_DIR/oodac/" 2>/dev/null || true
cp "$ROOT/cli/main.oo" "$DIST_DIR/cli/main.oo"
cp "$ROOT/scripts/oodac_pure_build.sh" "$DIST_DIR/scripts/"
cp "$ROOT/scripts/bootstrap_no_cargo.sh" "$DIST_DIR/scripts/"
cp "$ROOT/scripts/install_dress_rehearsal.sh" "$DIST_DIR/scripts/" 2>/dev/null || true
chmod +x "$DIST_DIR/scripts/"*.sh

# Runtime C (allowed forever)
cp "$ROOT/runtime/"*.c "$ROOT/runtime/"*.h "$DIST_DIR/runtime/" 2>/dev/null || true

if [[ -f install/install.oo ]]; then
  cp install/install.oo "$DIST_DIR/install/install.oo"
fi
if [[ -f install/BOOTSTRAP_PIN ]]; then
  cp install/BOOTSTRAP_PIN "$DIST_DIR/install/BOOTSTRAP_PIN"
fi

echo "$TAG" > "$DIST_DIR/share/VERSION"
cp README.md "$DIST_DIR/share/README.md" 2>/dev/null || true
[[ -f LICENSE ]] && cp LICENSE "$DIST_DIR/share/LICENSE"
[[ -f DESIGN.md ]] && cp DESIGN.md "$DIST_DIR/share/DESIGN.md"
[[ -f bootstrap/P4_DROPS.md ]] && cp bootstrap/P4_DROPS.md "$DIST_DIR/share/P4_DROPS.md"

cat > "$DIST_DIR/README.md" <<EOF
# openOODA ${TAG} (${ARCH})

Pure self-hosted release (no Rust/Cargo in product). **Not beta** unless owner tagged.

## Binaries
- \`bin/ooda\` — product CLI (pure .oo)
- \`oodac/oodac\` — compiler (pure .oo)

## Rebuild without rustc
\`\`\`
export SEED_OODAC=\$PWD/oodac/oodac
./scripts/bootstrap_no_cargo.sh
\`\`\`

Requires: gcc, bash, this seed tree.
EOF

mkdir -p "$ROOT/dist"
tar -C "$ROOT/dist" -czf "$TARBALL" "$NAME"
# Sidecar for CI pin verify (no secrets; publish with the tarball)
(
  cd "$ROOT/dist"
  sha256sum "$(basename "$TARBALL")" >"$(basename "$SHA_FILE")"
)
echo "[openOODA Release] wrote $TARBALL"
echo "[openOODA Release] wrote $SHA_FILE"
ls -la "$TARBALL" "$SHA_FILE"

# Offline dress (layout) when script present
if [[ -x "$ROOT/scripts/install_dress_rehearsal.sh" ]]; then
  RELEASE_TARBALL="$TARBALL" "$ROOT/scripts/install_dress_rehearsal.sh" \
    || echo "NOTE: dress rehearsal failed — fix before publish" >&2
fi

echo "release: PASSED (no cargo)"
echo "next: bootstrap/RELEASE_CHECKLIST.md (publish optional; no beta force)"
