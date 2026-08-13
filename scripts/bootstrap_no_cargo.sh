#!/usr/bin/env bash
# job: build product oodac + bin/ooda from seed
# in:  SEED_OODAC (or existing oodac/oodac|oodac2) + gcc + sources
# out: oodac/oodac, bin/ooda (pure .oo CLI)
#
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
if command -v shred >/dev/null 2>&1; then
  _SECRM="shred -u"
elif command -v srm >/dev/null 2>&1; then
  _SECRM="srm -z"
else
  _SECRM=""  # No secure-delete available; fall back to rm with warning
  echo "WARN: no secure-delete tool (shred/srm) available; using plain rm" >&2
fi
set -euo pipefail
# Default PURE_NO_ARC=0: keep retain/release in pure-built C (no strip required).
# Runtime release is leak-safe (does not free) until emit ARC is reclaim-correct.
# Optional PURE_NO_ARC=1 still strips if debugging seed-era heap issues.
export PURE_NO_ARC="${PURE_NO_ARC:-0}"
# PURE_SKIP_CHECK default 0: type-check self-round-trip is the backstop against
# a corrupt stage-1 emitter. Set to 1 only for explicit fast-path builds.
export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-0}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/bin" "$ROOT/oodac"

STAGE1="$ROOT/oodac/oodac"
# Prefer pinned cold seed over tree stage-1: a half-built/corrupt oodac/oodac
# must not become the emit host (ci deletes STAGE1 mid-bootstrap; SEGV host fails closed).
SEED_SRC="${SEED_OODAC:-}"
if [[ -z "$SEED_SRC" || ! -x "$SEED_SRC" ]]; then
  if [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then
    SEED_SRC="$ROOT/bootstrap/seed/oodac"
  elif [[ -x "$ROOT/oodac/oodac2" ]]; then
    SEED_SRC="$ROOT/oodac/oodac2"
  elif [[ -x "$ROOT/dist/ooda-v0.183.0-alpha-linux-x86_64/oodac/oodac" ]]; then
    SEED_SRC="$ROOT/dist/ooda-v0.183.0-alpha-linux-x86_64/oodac/oodac"
  elif [[ -x "$ROOT/oodac/oodac" ]]; then
    SEED_SRC="$ROOT/oodac/oodac"
  else
    echo "ERR_NO_SEED: set SEED_OODAC to a pure oodac binary" >&2
    echo "  (expected $ROOT/bootstrap/seed/oodac)" >&2
    exit 1
  fi
fi
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
if [[ ! -f "${SEED_SRC}.sha256" ]]; then
  echo "audit: no seed SHA sidecar — refusing" >&2
  exit 1
fi
# Compare hash field only (sidecar path may be basename or repo-relative).
_seed_expect="$(awk 'NF>=1 {print $1; exit}' "${SEED_SRC}.sha256")"
_seed_actual="$(sha256sum "$SEED_SRC" | awk '{print $1}')"
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
elif [[ ! -f "${SEED_SRC}.sha256.minisig" ]]; then
  _residual_why="no seed SHA signature (${SEED_SRC}.sha256.minisig)"
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
audit: SEED_PUB ($SEED_PUB), non-placeholder SEED_PUBKEY_FP, and ${SEED_SRC}.sha256.minisig
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
  if ! env -i PATH="/usr/bin:/bin" "$_MINISIGN" -V -p "$SEED_PUB" -m "${SEED_SRC}.sha256"; then
    echo "audit: minisign signature invalid — refusing" >&2
    exit 1
  fi
  echo "audit: seed SHA + minisign OK (fp=${SEED_PUBKEY_FP} verifier=${_MINISIGN})"
fi
trap '[[ -n "$_SECRM" ]] && $_SECRM "$SEED" 2>/dev/null; rm -f "$SEED"' EXIT
# Always copy seed aside so rm STAGE1 cannot unlink the live seed.
SEED="$TMPDIR/bootstrap_seed_oodac"
rm -f "$SEED"
umask 077
cp -a "$SEED_SRC" "$SEED"
chmod 700 "$SEED"
echo "bootstrap: seed=$SEED (from $SEED_SRC)"

# Residual honesty: tree stage-1 can still SEGV as *emit host* on some modules.
# Cold seed remains the trusted emit host for pure multi. Stage-1 is the product
# oodac binary. Native prove = build+exec (not interpreter run).
#
# 1) Rebuild oodac from sources (pure multi) — seed is emit host
rm -f "$STAGE1"
echo "=== seed builds oodac (emit host=seed) ==="
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "scripts/oodac_pure_build.sh" "$ROOT/oodac/main.oo" "$STAGE1")
if [[ ! -x "$STAGE1" ]]; then
  echo "FAIL: seed did not produce $STAGE1" >&2
  exit 1
fi

# 2) Build product CLI from cli/main.oo — seed is emit host (not stage-1)
CLI_OUT="$ROOT/bin/ooda"
rm -f "$CLI_OUT"
echo "=== seed builds pure .oo product CLI (emit host=seed) ==="
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "scripts/oodac_pure_build.sh" "$ROOT/cli/main.oo" "$CLI_OUT")
if [[ ! -x "$CLI_OUT" ]]; then
  echo "FAIL: pure CLI missing at $CLI_OUT" >&2
  exit 1
fi

# 3) Smoke product CLI
echo "=== smoke product bin/ooda ==="
"$CLI_OUT" version | tee "$TMPDIR/bootstrap_ver.txt"
grep -q '0.184.4-alpha' "$TMPDIR/bootstrap_ver.txt"
"$CLI_OUT" check "$ROOT/fixtures/chs_list_string.oo" | tee "$TMPDIR/bootstrap_chk.txt"
grep -qE '^OK' "$TMPDIR/bootstrap_chk.txt"
SMOKE_BIN="$TMPDIR/bootstrap_chs_native"
rm -f "$SMOKE_BIN"
# Emit host = seed (same residual policy as pure builds above).
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "$CLI_OUT" build \
  "$ROOT/fixtures/chs_list_string.oo" -o "$SMOKE_BIN")
if [[ ! -x "$SMOKE_BIN" ]]; then
  echo "FAIL: product build did not produce $SMOKE_BIN" >&2
  exit 1
fi
"$SMOKE_BIN" | tee "$TMPDIR/bootstrap_run.txt"
grep -q '2' "$TMPDIR/bootstrap_run.txt"

# Product purity: report residual .rs count (B0 wants 0)
RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' 2>/dev/null | wc -l)
echo "RS_COUNT=$RS"
echo "bootstrap: PASSED"
echo "  oodac: $STAGE1"
echo "  ooda:  $CLI_OUT"
exit 0
