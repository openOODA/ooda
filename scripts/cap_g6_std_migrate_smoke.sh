#!/usr/bin/env bash
# CAP-G6: std migration smoke — path-A wave gate for product facades
#
# Criteria (exit 0 when path-A wave met):
#   1. Report remaining bare &NetCap / &FsCap in monorepo std/src
#   2. FAIL if product facade http_get is still NetCap-only (must be &HttpCap)
#   3. FAIL if fs_read_file still takes &FsCap (must be &FsReadCap)
#   4. PASS with residual note if SysCap bulk remains (not a hard gate)
#
# No push. Honesty: bulk domain SysCap stubs are residual, not greenwashed.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Resolve monorepo / product roots from either std/scripts or ooda/scripts.
if [[ -d "$SCRIPT_DIR/../src" && -f "$SCRIPT_DIR/../src/net.oo" ]]; then
  # Running from monorepo std/scripts
  STD_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
  MONO_ROOT="$(cd "$STD_ROOT/.." && pwd)"
  OODA_ROOT="${OODA_ROOT:-$MONO_ROOT/ooda}"
elif [[ -d "$SCRIPT_DIR/../std" ]]; then
  # Running from ooda/scripts
  OODA_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
  MONO_ROOT="$(cd "$OODA_ROOT/.." && pwd)"
  STD_ROOT="${STD_ROOT:-$MONO_ROOT/std}"
else
  echo "ERR: cannot resolve std/ooda roots from $SCRIPT_DIR" >&2
  exit 1
fi

STD_SRC="${STD_SRC:-$STD_ROOT/src}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
note() { echo "NOTE $*"; }

# ---------------------------------------------------------------------------
# 1) Count remaining bare &NetCap / &FsCap in monorepo std/src (report)
# ---------------------------------------------------------------------------
if [[ ! -d "$STD_SRC" ]]; then
  bad "missing std/src at $STD_SRC"
else
  NET_COUNT="$(grep -RIn --include='*.oo' -e '&NetCap' "$STD_SRC" 2>/dev/null | wc -l | tr -d ' ')"
  FS_COUNT="$(grep -RIn --include='*.oo' -e '&FsCap' "$STD_SRC" 2>/dev/null | wc -l | tr -d ' ')"
  SYS_COUNT="$(grep -RIn --include='*.oo' -e '&SysCap' "$STD_SRC" 2>/dev/null | wc -l | tr -d ' ')"
  HTTP_COUNT="$(grep -RIn --include='*.oo' -e '&HttpCap' "$STD_SRC" 2>/dev/null | wc -l | tr -d ' ')"
  FSREAD_COUNT="$(grep -RIn --include='*.oo' -e '&FsReadCap' "$STD_SRC" 2>/dev/null | wc -l | tr -d ' ')"

  echo "REPORT std/src bare-cap residual counts:"
  echo "  &NetCap     = $NET_COUNT"
  echo "  &FsCap      = $FS_COUNT"
  echo "  &HttpCap    = $HTTP_COUNT"
  echo "  &FsReadCap  = $FSREAD_COUNT"
  echo "  &SysCap     = $SYS_COUNT"
  pass "reported bare &NetCap=$NET_COUNT &FsCap=$FS_COUNT in std/src"
fi

# ---------------------------------------------------------------------------
# 2) Product facades: net.oo http_get must be HttpCap (not NetCap-only)
# ---------------------------------------------------------------------------
NET_FACADES=(
  "$STD_SRC/net.oo"
  "$OODA_ROOT/std/os/net.oo"
  "$OODA_ROOT/std/src/net.oo"
)

http_get_sig() {
  # Extract http_get formal list (single-line pub fn form)
  grep -E '^\s*pub\s+fn\s+http_get\s*\(' "$1" 2>/dev/null | head -1 || true
}

checked_net=0
for f in "${NET_FACADES[@]}"; do
  [[ -f "$f" ]] || continue
  checked_net=1
  sig="$(http_get_sig "$f")"
  if [[ -z "$sig" ]]; then
    bad "no pub fn http_get in $f"
    continue
  fi
  # Require HttpCap on the http_get formal; NetCap-only is a path-A fail.
  if echo "$sig" | grep -qE '&HttpCap'; then
    if echo "$sig" | grep -qE '&NetCap'; then
      bad "http_get still mentions &NetCap in $f: $sig"
    else
      pass "http_get uses &HttpCap ($f)"
    fi
  elif echo "$sig" | grep -qE '&NetCap'; then
    bad "product facade http_get still NetCap-only ($f): $sig"
  else
    bad "http_get missing &HttpCap formal ($f): $sig"
  fi
done
if [[ $checked_net -eq 0 ]]; then
  bad "no product net.oo facade found under $STD_SRC or $OODA_ROOT/std"
fi

# ---------------------------------------------------------------------------
# 3) Product facades: fs_read_file must not still take &FsCap
# ---------------------------------------------------------------------------
FS_FACADES=(
  "$STD_SRC/fs.oo"
  "$OODA_ROOT/std/os/fs.oo"
  "$OODA_ROOT/std/src/fs.oo"
)

fs_read_sig() {
  grep -E '^\s*pub\s+fn\s+fs_read_file\s*\(' "$1" 2>/dev/null | head -1 || true
}

checked_fs=0
for f in "${FS_FACADES[@]}"; do
  [[ -f "$f" ]] || continue
  checked_fs=1
  sig="$(fs_read_sig "$f")"
  if [[ -z "$sig" ]]; then
    bad "no pub fn fs_read_file in $f"
    continue
  fi
  if echo "$sig" | grep -qE '&FsCap([^A-Za-z]|$)'; then
    bad "fs_read_file still &FsCap ($f): $sig"
  elif echo "$sig" | grep -qE '&FsReadCap'; then
    pass "fs_read_file uses &FsReadCap ($f)"
  else
    bad "fs_read_file missing &FsReadCap formal ($f): $sig"
  fi
done
if [[ $checked_fs -eq 0 ]]; then
  bad "no product fs.oo facade found under $STD_SRC or $OODA_ROOT/std"
fi

# ---------------------------------------------------------------------------
# 4) SysCap bulk residual — pass with honesty note (not a hard fail)
# ---------------------------------------------------------------------------
SYS_COUNT="${SYS_COUNT:-0}"
if [[ "${SYS_COUNT}" -gt 0 ]]; then
  note "residual: &SysCap bulk remains in std/src (count=$SYS_COUNT) — CAP-G6 path-A does not require full SysCap purge"
  pass "SysCap bulk residual noted (not a gate)"
else
  pass "no bare &SysCap in std/src"
fi

# Soft: optional ci_product wire (do not fail product if absent)
CI_PRODUCT="$OODA_ROOT/scripts/ci_product.sh"
if [[ -f "$CI_PRODUCT" ]] && grep -q 'cap_g6_std_migrate_smoke' "$CI_PRODUCT" 2>/dev/null; then
  pass "ci_product soft-wire present"
else
  pass "ci_product soft-wire residual (optional)"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_g6_std_migrate_smoke: FAILED" >&2
  exit 1
fi
echo "cap_g6_std_migrate_smoke: PASSED (path-A wave criteria met)"
exit 0
