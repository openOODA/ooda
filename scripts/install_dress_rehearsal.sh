#!/usr/bin/env bash
# job: pure-product offline install dress rehearsal — validate release layout
# in:  RELEASE_TARBALL | RELEASE_TREE | auto-pick dist/ from BOOTSTRAP_PIN / release.sh
# out: exit 0 if layout green; optional binary smoke when executables present
# Offline clean-machine sim; pure product layout only; does not tag beta.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

PIN_FILE="$ROOT/install/BOOTSTRAP_PIN"
PIN="v0.186.0-alpha"
if [[ -f "$PIN_FILE" ]]; then
  PIN="$(tr -d '\r\n' <"$PIN_FILE" | head -1)"
  case "$PIN" in v*) ;; *) PIN="v${PIN}" ;; esac
fi

ARCH="linux-x86_64"
NAME="ooda-${PIN}-${ARCH}"
DEFAULT_TB="$ROOT/dist/${NAME}.tar.gz"
DEFAULT_TREE="$ROOT/dist/${NAME}"

TREE=""
CLEANUP=""
resolve_tree() {
  if [[ -n "${RELEASE_TREE:-}" ]]; then
    TREE="$RELEASE_TREE"
    return
  fi
  if [[ -n "${RELEASE_TARBALL:-}" ]]; then
    local tb="$RELEASE_TARBALL"
    [[ -f "$tb" ]] || { bad "RELEASE_TARBALL missing: $tb"; return; }
    CLEANUP="$TMPDIR/dress_extract_$$"
    rm -rf "$CLEANUP"
    mkdir -p "$CLEANUP"
    tar -C "$CLEANUP" -xzf "$tb"
    TREE="$CLEANUP/$(ls "$CLEANUP" | head -1)"
    return
  fi
  if [[ -d "$DEFAULT_TREE" ]]; then
    TREE="$DEFAULT_TREE"
    return
  fi
  if [[ -f "$DEFAULT_TB" ]]; then
    RELEASE_TARBALL="$DEFAULT_TB" resolve_tree
    return
  fi
  # Last resort: stage from working tree pure binaries (dev dress, not release ship)
  if [[ -x "$ROOT/bin/ooda" && -x "$ROOT/oodac/oodac" ]]; then
    CLEANUP="$TMPDIR/dress_stage_$$"
    rm -rf "$CLEANUP"
    mkdir -p "$CLEANUP/bin" "$CLEANUP/oodac" "$CLEANUP/share" \
      "$CLEANUP/runtime" "$CLEANUP/install" "$CLEANUP/scripts" "$CLEANUP/cli"
    cp "$ROOT/bin/ooda" "$CLEANUP/bin/ooda"
    cp "$ROOT/oodac/oodac" "$CLEANUP/oodac/oodac"
    echo "$PIN" >"$CLEANUP/share/VERSION"
    cp "$ROOT/runtime/"*.c "$ROOT/runtime/"*.h "$CLEANUP/runtime/" 2>/dev/null || true
    [[ -f "$ROOT/install/install.oo" ]] && cp "$ROOT/install/install.oo" "$CLEANUP/install/"
    [[ -f "$ROOT/scripts/bootstrap_no_cargo.sh" ]] && cp "$ROOT/scripts/bootstrap_no_cargo.sh" "$CLEANUP/scripts/"
    TREE="$CLEANUP"
    echo "NOTE: no dist tarball; staged from working tree (run scripts/release.sh for real ship)"
    return
  fi
  bad "no RELEASE_TREE, RELEASE_TARBALL, dist/${NAME}, or local bin+oodac"
}

resolve_tree
if [[ -z "$TREE" || ! -d "$TREE" ]]; then
  echo "install_dress_rehearsal: FAILED (no tree)" >&2
  exit 1
fi
echo "dress tree: $TREE"
echo "pin: $PIN"

# --- layout rails (clean machine expectations) ---
[[ -x "$TREE/bin/ooda" ]] && pass "bin/ooda executable" || bad "bin/ooda missing/not +x"
[[ -x "$TREE/oodac/oodac" ]] && pass "oodac/oodac executable" || bad "oodac/oodac missing/not +x"
[[ -f "$TREE/share/VERSION" ]] && pass "share/VERSION" || bad "share/VERSION missing"

if [[ -f "$TREE/share/VERSION" ]]; then
  VER="$(tr -d '\r\n' <"$TREE/share/VERSION" | head -1)"
  case "$VER" in
    "$PIN"|"${PIN#v}") pass "VERSION=$VER matches pin" ;;
    *) bad "VERSION=$VER does not match pin $PIN" ;;
  esac
fi

[[ -f "$TREE/runtime/chs_rt.c" || -f "$TREE/runtime/chs_rt.h" ]] \
  && pass "runtime C present" || bad "runtime C missing"
[[ ! -f "$TREE/Cargo.toml" ]] && pass "no Cargo.toml in release" || bad "Cargo.toml in release"
RS=$(find "$TREE" -name '*.rs' 2>/dev/null | wc -l)
[[ "$RS" -eq 0 ]] && pass "RS=0 in release tree" || bad "RS=$RS in release tree"

# --- optional smoke (no network) ---
if [[ -x "$TREE/bin/ooda" ]]; then
  set +e
  "$TREE/bin/ooda" version >"$TMPDIR/dress_ver.txt" 2>"$TMPDIR/dress_ver.err"
  rv=$?
  set -e
  if [[ $rv -eq 0 ]] && grep -qiE 'ooda|pure|alpha' "$TMPDIR/dress_ver.txt"; then
    pass "bin/ooda version smoke"
    cat "$TMPDIR/dress_ver.txt"
  else
    bad "bin/ooda version failed"
    cat "$TMPDIR/dress_ver.err" 2>/dev/null || true
  fi
fi

# Anti: no cargo on dress path
if grep -nE '^[[:space:]]*(cargo|rustc)([[:space:]]|$)' "$0" | grep -vE 'grep|comment|#' ; then
  bad "dress script invokes cargo/rustc"
else
  pass "dress script never invokes cargo/rustc"
fi

if [[ -n "$CLEANUP" && -d "$CLEANUP" ]]; then
  rm -rf "$CLEANUP"
fi

if [[ $fail -ne 0 ]]; then
  echo "install_dress_rehearsal: FAILED" >&2
  exit 1
fi
echo "install_dress_rehearsal: PASSED"
echo "residual: full XDG install.oo network fetch not exercised offline; layout+version only"
exit 0
