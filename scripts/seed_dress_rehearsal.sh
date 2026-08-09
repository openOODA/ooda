#!/usr/bin/env bash
# job: offline cold-start seed dress rehearsal
# in:  bootstrap/seed/oodac or SEED_OODAC (optional); gcc + sources when seed present
# out: exit 0 if seed present and bootstrap_no_cargo green; or honest skip residual
#      exit 1 if bootstrap fails, or if SEED_REQUIRED=1 and no seed
# Anti: never cargo/rustc; never network; never beta tag
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

# --- resolve seed (offline only; no release download) ---
SEED=""
if [[ -n "${SEED_OODAC:-}" && -x "${SEED_OODAC}" ]]; then
  SEED="$SEED_OODAC"
  echo "seed_dress_rehearsal: SEED_OODAC=$SEED"
elif [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then
  SEED="$ROOT/bootstrap/seed/oodac"
  echo "seed_dress_rehearsal: bootstrap/seed/oodac"
fi

if [[ -z "$SEED" ]]; then
  msg="residual: no offline seed (place bootstrap/seed/oodac or set SEED_OODAC); see bootstrap/seed/README.md"
  if [[ "${SEED_REQUIRED:-0}" == "1" ]]; then
    echo "seed_dress_rehearsal: FAILED ($msg)" >&2
    exit 1
  fi
  echo "seed_dress_rehearsal: SKIP"
  echo "$msg"
  echo "tip: SEED_REQUIRED=1 forces fail-closed when seed is absent"
  exit 0
fi

# Optional integrity note (non-fatal if sidecar absent)
if [[ -f "${SEED}.sha256" ]]; then
  if (cd "$(dirname "$SEED")" && sha256sum -c "$(basename "$SEED").sha256") >/dev/null 2>&1; then
    echo "OK sha256 sidecar matches"
  else
    echo "WARN: ${SEED}.sha256 present but does not match binary" >&2
  fi
fi

echo "=== bootstrap_no_cargo with seed ==="
export SEED_OODAC="$SEED"
if ! "$ROOT/scripts/bootstrap_no_cargo.sh"; then
  echo "seed_dress_rehearsal: FAILED (bootstrap_no_cargo)" >&2
  exit 1
fi

# Light product smoke (bootstrap already smokes; re-state for dress honesty)
if [[ ! -x "$ROOT/bin/ooda" ]]; then
  echo "seed_dress_rehearsal: FAILED (bin/ooda missing after bootstrap)" >&2
  exit 1
fi
"$ROOT/bin/ooda" version | tee "$TMPDIR/seed_dress_ver.txt"
grep -qE 'ooda|alpha|0\.' "$TMPDIR/seed_dress_ver.txt"

echo "seed_dress_rehearsal: PASSED"
echo "  seed: $SEED"
echo "  oodac: $ROOT/oodac/oodac"
echo "  ooda:  $ROOT/bin/ooda"
echo "related: scripts/install_dress_rehearsal.sh (release layout); bootstrap/GHA_PRODUCT.md (remote CI)"
exit 0
