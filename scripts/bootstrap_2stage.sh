#!/usr/bin/env bash
# Trust anchor: hardcoded minisign public-key fingerprint (NOT the sidecar).
# Anyone who can rewrite the .sha256 sidecar can substitute the seed; the
# fingerprint below is the published root the build refuses to bypass.
# Fingerprint = minisign key id from oodac.pub untrusted comment
#   ("untrusted comment: minisign public key <KEY_ID>"). Real gate is minisign -V.
# Residual escape (dev/research only, never product trust):
#   OODA_SEED_ALLOW_UNSIGNED=1  — loud WARN, still prefer sha256sum -c
SEED_PUBKEY_FP="645069A34E6058B7"
# Full minisign pubkey body (line 2 of oodac.pub) — real pin; comment key id is untrusted.
SEED_PUBKEY_B64="RWS3WGBOo2lQZGh3eFa0Gq0h6vDb9rCE5ZoaExEGdSko44OiPDdTVm8i"
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/bin" "$ROOT/oodac"

SEED="$ROOT/bootstrap/seed/oodac"
STAGE1="$ROOT/oodac/stage1_noarc"
STAGE2="$ROOT/oodac/oodac"

SEED_PUB="${SEED_PUB:-$ROOT/bootstrap/seed/oodac.pub}"
# minisign resolver (supply-chain): absolute paths first, then tools/, then PATH.
# Prefer abs so attacker-controlled PATH cannot inject a fake verifier.
_MINISIGN=""
for _ms in /usr/bin/minisign /bin/minisign; do
  if [[ -x "$_ms" ]]; then
    _MINISIGN="$_ms"
    break
  fi
done
if [[ -z "$_MINISIGN" && -x "$ROOT/tools/minisign" ]]; then
  _MINISIGN="$ROOT/tools/minisign"
fi
if [[ -z "$_MINISIGN" ]] && command -v minisign >/dev/null 2>&1; then
  _MINISIGN="$(command -v minisign)"
fi
# Audit: seed-binary verification before the seed is used in the bootstrap chain.
# Integrity (sha256) always preferred; minisign is product trust (fail closed).
if [[ ! -f "${SEED}.sha256" ]]; then
  echo "audit: no seed SHA sidecar — refusing" >&2
  exit 1
fi
# Compare hash field only (sidecar path may be basename or repo-relative).
_seed_expect="$(awk 'NF>=1 {print $1; exit}' "${SEED}.sha256")"
_seed_actual="$(sha256sum "$SEED" | awk '{print $1}')"
if [[ -z "$_seed_expect" || "$_seed_expect" != "$_seed_actual" ]]; then
  echo "audit: seed SHA mismatch — refusing" >&2
  echo "audit: expected ${_seed_expect:-<empty>} actual ${_seed_actual}" >&2
  exit 1
fi
echo "audit: seed SHA OK"

_residual_why=""
if [[ "$SEED_PUBKEY_FP" == "TODO_PUBKEY_FP" || -z "$SEED_PUBKEY_FP" ]]; then
  _residual_why="SEED_PUBKEY_FP is still placeholder TODO_PUBKEY_FP"
elif [[ ! -f "$SEED_PUB" ]]; then
  _residual_why="SEED_PUB missing: $SEED_PUB"
elif [[ -z "$_MINISIGN" ]]; then
  _residual_why="minisign not on PATH and no $ROOT/tools/minisign"
elif [[ ! -f "${SEED}.sha256.minisig" ]]; then
  _residual_why="no seed SHA signature (${SEED}.sha256.minisig)"
fi

if [[ -n "$_residual_why" ]]; then
  if [[ "${OODA_SEED_ALLOW_UNSIGNED:-0}" == "1" ]]; then
    cat >&2 <<'EOF'
WARN: ============================================================
WARN: OODA_SEED_ALLOW_UNSIGNED=1 — seed signature verification SKIPPED
WARN: This is a residual / dev-research escape hatch, NOT product trust.
WARN: Anyone who can rewrite the seed (or its .sha256) can substitute code.
WARN: For product trust: install minisign (or tools/minisign), publish
WARN: oodac.pub + .minisig, set SEED_PUBKEY_FP (see bootstrap/seed/SIGNING.oot).
WARN: ============================================================
EOF
    echo "audit: residual reason: ${_residual_why}" >&2
    echo "audit: sha256 sidecar checked; minisign SKIPPED (unsigned residual)" >&2
  else
    cat >&2 <<EOF
audit: residual seed signing — refusing (${_residual_why})
audit: product path requires: minisign on PATH or \$ROOT/tools/minisign,
audit: SEED_PUB ($SEED_PUB), non-placeholder SEED_PUBKEY_FP, and ${SEED}.sha256.minisig
audit: see bootstrap/seed/SIGNING.oot
audit: dev/research only: OODA_SEED_ALLOW_UNSIGNED=1 (loud skip, not trust)
EOF
    exit 1
  fi
else
  # Pin pubkey *body* (base64 line), not only untrusted-comment key id.
  # Comment key id is attacker-writable; body is what minisign -V actually uses.
  _pub_body="$(awk '!/^untrusted comment/ && NF { print $1; exit }' "$SEED_PUB")"
  _pub_fp="$(awk '/^untrusted comment: minisign public key / { print $NF; exit }' "$SEED_PUB")"
  if [[ -z "$_pub_body" || "$_pub_body" != "$SEED_PUBKEY_B64" ]]; then
    echo "audit: SEED_PUB body mismatch — refusing (pinned pubkey body != $SEED_PUB)" >&2
    echo "audit: expected SEED_PUBKEY_B64 from bootstrap script; pub may be substituted" >&2
    exit 1
  fi
  if [[ -n "$_pub_fp" && "$_pub_fp" != "$SEED_PUBKEY_FP" ]]; then
    echo "audit: SEED_PUBKEY_FP mismatch — refusing (script=${SEED_PUBKEY_FP} comment=${_pub_fp})" >&2
    exit 1
  fi
  # Clean env for verifier: drop LD_PRELOAD / LD_LIBRARY_PATH / gadget vars (T3 parity).
  if ! env -i PATH="/usr/bin:/bin" "$_MINISIGN" -V -p "$SEED_PUB" -m "${SEED}.sha256"; then
    echo "audit: minisign signature invalid — refusing" >&2
    exit 1
  fi
  echo "audit: seed SHA + minisign OK (fp=${SEED_PUBKEY_FP} verifier=${_MINISIGN})"
fi

echo "=== STAGE 1: seed builds stage1_noarc (PURE_NO_ARC=1) ==="
(cd "$ROOT" && env -u OODA PURE_NO_ARC=1 OODAC_BIN="$SEED" "$SEED" build "$ROOT/oodac/main.oo" "$STAGE1")
if [[ ! -x "$STAGE1" ]]; then
  echo "FAIL: STAGE1" >&2
  exit 1
fi

echo "=== STAGE 2: stage1_noarc builds oodac (PURE_NO_ARC=0) ==="
(cd "$ROOT" && env -u OODA PURE_NO_ARC=0 OODAC_BIN="$STAGE1" "$STAGE1" build "$ROOT/oodac/main.oo" "$STAGE2")
if [[ ! -x "$STAGE2" ]]; then
  echo "FAIL: STAGE2" >&2
  exit 1
fi

echo "=== Build pure CLI with STAGE2 (PURE_NO_ARC=0) ==="
CLI_OUT="$ROOT/bin/ooda"
(cd "$ROOT" && env -u OODA PURE_NO_ARC=0 OODAC_BIN="$STAGE2" "$STAGE2" build "$ROOT/cli/main.oo" "$CLI_OUT")

echo "=== arc_smoke.sh ==="
export OODAC_BIN="$STAGE2"
bash "$ROOT/scripts/arc_smoke.sh"
echo "DONE!"
