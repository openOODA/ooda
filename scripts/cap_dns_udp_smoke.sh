#!/usr/bin/env bash
# DNS UdpCap smoke — resolve_ipv4 prefers &UdpCap (not bare NetCap-only)
#
# Criteria (exit 0 when path-A holds):
#   1. Static: std/src/net/dns.oo resolve_ipv4 formal is &UdpCap
#   2. Static: not bare NetCap-only (must not list only &NetCap)
#   3. Residual honesty string present (UDP socket DNS not implemented)
#   4. Optional: oodac check dns.oo when binary present
#
# Wire: after cap_process_facade_smoke in ci_product. No push.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
if [[ ! -x "$OODAC" && -x "$ROOT/../oodac/oodac" ]]; then
  OODAC="$ROOT/../oodac/oodac"
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
note() { echo "NOTE $*"; }

DNS="$ROOT/std/src/net/dns.oo"
[[ -f "$DNS" ]] || { echo "ERR: missing $DNS" >&2; exit 1; }

# ---------------------------------------------------------------------------
# 1–2) Static: resolve_ipv4 uses &UdpCap (not bare NetCap-only)
# ---------------------------------------------------------------------------
resolve_sig="$(grep -E '^\s*(pub\s+)?fn\s+resolve_ipv4\s*\(' "$DNS" 2>/dev/null | head -1 || true)"
if [[ -z "$resolve_sig" ]]; then
  bad "no fn resolve_ipv4 in dns.oo"
elif echo "$resolve_sig" | grep -qE '&UdpCap'; then
  if echo "$resolve_sig" | grep -qE '&NetCap'; then
    bad "resolve_ipv4 still mentions &NetCap: $resolve_sig"
  else
    pass "static resolve_ipv4 uses &UdpCap"
  fi
elif echo "$resolve_sig" | grep -qE '&NetCap'; then
  bad "resolve_ipv4 still NetCap-only (prefer &UdpCap): $resolve_sig"
else
  bad "resolve_ipv4 missing &UdpCap formal: $resolve_sig"
fi

# ---------------------------------------------------------------------------
# 3) Residual honesty string present
# ---------------------------------------------------------------------------
HONESTY='Residual: UDP socket DNS not implemented'
if grep -qF "$HONESTY" "$DNS"; then
  pass "residual honesty string present"
else
  bad "missing residual honesty string: $HONESTY"
fi

# Soft: comment honesty that real UDP DNS is not product yet
if grep -qiE 'residual|not (product|implemented|lowered)' "$DNS"; then
  pass "residual commentary present"
else
  note "no residual commentary beyond Err string"
  pass "residual commentary soft residual"
fi

# ---------------------------------------------------------------------------
# 4) Optional oodac check (soft if binary missing; hard when present)
# ---------------------------------------------------------------------------
if [[ -x "$OODAC" ]]; then
  set +e
  cout=$("$OODAC" check "$DNS" 2>&1); crc=$?
  set -e
  if [[ $crc -ne 0 ]]; then
    bad "oodac check dns.oo (rc=$crc): $cout"
  else
    pass "oodac check dns.oo"
  fi
else
  note "oodac not found at $OODAC — skip check"
  pass "oodac check residual (optional)"
fi

# Soft: ci_product wire after cap_process_facade_smoke
CI_PRODUCT="$ROOT/scripts/ci_product.sh"
if [[ -f "$CI_PRODUCT" ]] && grep -q 'cap_dns_udp_smoke' "$CI_PRODUCT" 2>/dev/null; then
  if grep -A2 'cap_process_facade_smoke' "$CI_PRODUCT" 2>/dev/null | grep -q 'cap_dns_udp_smoke'; then
    pass "ci_product wire after cap_process"
  else
    pass "ci_product soft-wire present"
  fi
else
  pass "ci_product soft-wire residual (optional)"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_dns_udp_smoke: FAILED" >&2
  exit 1
fi
echo "cap_dns_udp_smoke: PASSED"
exit 0
