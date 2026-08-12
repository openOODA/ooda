#!/usr/bin/env bash
# CAP-G1: granular net least-privilege check (Http/Tcp/Udp/Bind + NetCap supersede)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/cap_g1_smoke_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

pass() { echo "OK $*"; }
fail() { echo "FAIL $*" >&2; exit 1; }

[[ -x "$OODAC" ]] || fail "oodac not executable: $OODAC"

# Static: preferred map present
grep -q 'sealed_net_kind_of' oodac/check_cap_util.oo || fail "missing sealed_net_kind_of"
grep -q 'return "BindCap"' oodac/check_cap_util.oo || fail "tcp_bind not BindCap preferred"
grep -q 'oo_cap_require_http\|require_http' runtime/chs_rt_sys.c runtime/chs_rt_netfloor.c 2>/dev/null \
  || grep -q 'oo_cap_require_http' runtime/chs_rt_sys.c || fail "missing require_http"
pass "static CAP-G1 markers"

check_deny() {
  local name=$1 body=$2
  printf '%s\n' "$body" >"$TMP/$name.oo"
  set +e
  out=$("$OODAC" check "$TMP/$name.oo" 2>&1)
  rc=$?
  set -e
  if [[ $rc -eq 0 ]] && ! echo "$out" | grep -qiE 'ERR|Capability|cap'; then
    fail "check accepted deny $name"
  fi
  pass "deny $name (rc=$rc)"
}

check_allow() {
  local name=$1 body=$2
  printf '%s\n' "$body" >"$TMP/$name.oo"
  set +e
  out=$("$OODAC" check "$TMP/$name.oo" 2>&1)
  rc=$?
  set -e
  if [[ $rc -ne 0 ]] || echo "$out" | grep -qiE 'Capability Violation|requires a &'; then
    fail "check rejected allow $name (rc=$rc): $out"
  fi
  pass "allow $name"
}

check_deny http_tcp_bind 'fn main(h: &HttpCap) { let _ = tcp_bind(h, 8080); }'
check_deny tcp_fetch 'fn main(t: &TcpCap) { let _ = fetch(t, "http://x/"); }'
check_allow net_tcp_bind 'fn main(net: &NetCap) { let _ = tcp_bind(net, 8080); }'
check_allow http_fetch 'fn main(h: &HttpCap) { let _ = fetch(h, "http://x/"); }'
check_allow tcp_connect 'fn main(t: &TcpCap) { let _ = tcp_connect(t, "127.0.0.1", 80); }'
check_allow bind_only 'fn main(b: &BindCap) { let _ = tcp_bind(b, 8080); }'

pass "ci_product optional: wire cap_g1_net_granular_smoke"
echo "cap_g1_net_granular_smoke: PASSED"
