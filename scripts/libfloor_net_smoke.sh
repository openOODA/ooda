#!/usr/bin/env bash
# M161 libfloor path A — NetCap tcp_bind/connect, bind_udp, tls_connect residual seals
# Dual path: granted → Result residual Err; forge (zero/magic) → ERR cap deny
# Honesty: no real sockets / TLS handshake — path A seal + residual only
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm)

# check: bare tcp_connect without NetCap refused
cat >"$TMPDIR/lfn_bare.oo" <<'EOF'
pub fn main() { let r = tcp_connect("127.0.0.1", 1); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/lfn_bare.oo" >"$TMPDIR/lfn_bare.out" 2>"$TMPDIR/lfn_bare.err"
brc=$?
set -e
if [[ $brc -ne 0 ]] && grep -qiE 'capability|NetCap|ERR' "$TMPDIR/lfn_bare.out" "$TMPDIR/lfn_bare.err" 2>/dev/null; then
  pass "check refuse bare tcp_connect"
else
  bad "bare tcp_connect accepted rc=$brc"
fi

# check: granted NetCap fixtures
set +e
"$OODAC_BIN" check "$ROOT/fixtures/libfloor_tcp.oo" >"$TMPDIR/lfn_ck_tcp.out" 2>"$TMPDIR/lfn_ck_tcp.err"
trc=$?
"$OODAC_BIN" check "$ROOT/fixtures/libfloor_tls.oo" >"$TMPDIR/lfn_ck_tls.out" 2>"$TMPDIR/lfn_ck_tls.err"
lrc=$?
set -e
[[ $trc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/lfn_ck_tcp.out" && pass "check tcp_connect with NetCap" || bad "check libfloor_tcp"
[[ $lrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/lfn_ck_tls.out" && pass "check tls_connect with NetCap" || bad "check libfloor_tls"

# std thin wrappers check
set +e
"$OODAC_BIN" check "$ROOT/std/os/net.oo" >"$TMPDIR/lfn_std.out" 2>"$TMPDIR/lfn_std.err"
src=$?
set -e
[[ $src -eq 0 ]] && grep -qE '^OK' "$TMPDIR/lfn_std.out" && pass "check std/os/net.oo" || {
  bad "check std/os/net.oo"
  head -12 "$TMPDIR/lfn_std.out" "$TMPDIR/lfn_std.err" 2>/dev/null || true
}

emit_run() {
  # $1=fixture $2=binbase $3=needle $4=label $5=oo_symbol
  local fix="$1" base="$2" needle="$3" label="$4" sym="$5"
  set +e
  "$OODAC_BIN" emit-c "$fix" >"$TMPDIR/${base}.c" 2>"$TMPDIR/${base}.err"
  local erc=$?
  set -e
  if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/${base}.c" "$TMPDIR/${base}.err" 2>/dev/null; then
    bad "emit-c $label"
    head -15 "$TMPDIR/${base}.err" "$TMPDIR/${base}.c" || true
    return
  fi
  pass "emit-c $label"
  if ! grep -qE "${sym}\\(" "$TMPDIR/${base}.c"; then
    bad "emit missing $sym"
  else
    pass "emit lowers $sym"
  fi
  if ! grep -q 'oo_cap_grant_net' "$TMPDIR/${base}.c"; then
    bad "emit missing grant_net ($label)"
  else
    pass "emit grant_net ($label)"
  fi
  # honesty: generated C must not call real sockets/TLS (not oo_tcp_connect etc.)
  if grep -qE '\bsocket\(|\bconnect\(|\bbind\(|\blisten\(|\bSSL_|\bgetaddrinfo\(' "$TMPDIR/${base}.c"; then
    bad "emit real socket/TLS symbols ($label)"
  else
    pass "emit honesty no socket/TLS ($label)"
  fi
  gcc "${RT[@]}" "$TMPDIR/${base}.c" -o "$TMPDIR/${base}.bin" 2>"$TMPDIR/${base}_gcc.err" || {
    bad "gcc $label"
    head -20 "$TMPDIR/${base}_gcc.err" || true
    return
  }
  local out
  out=$("$TMPDIR/${base}.bin" 2>&1) || true
  if echo "$out" | grep -q "$needle"; then
    pass "runtime residual $label"
  else
    bad "runtime $label out=$out"
  fi
  # forge deny: zero NetCap
  sed -E 's/long long net = oo_cap_grant_net\(\)/long long net = 0LL/' \
    "$TMPDIR/${base}.c" >"$TMPDIR/${base}_zero.c"
  gcc "${RT[@]}" "$TMPDIR/${base}_zero.c" -o "$TMPDIR/${base}_zero.bin" -lm
  set +e
  local zout zrc=0
  zout=$("$TMPDIR/${base}_zero.bin" 2>&1) || zrc=$?
  set -e
  if [[ $zrc -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
    pass "zero forge deny $label"
  else
    bad "zero forge $label out=$zout rc=$zrc"
  fi
  # classic magic forge NetCap 0x4F4F4E54 "OONT"
  sed -E 's/long long net = oo_cap_grant_net\(\)/long long net = 0x4F4F4E54LL/' \
    "$TMPDIR/${base}.c" >"$TMPDIR/${base}_mag.c"
  gcc "${RT[@]}" "$TMPDIR/${base}_mag.c" -o "$TMPDIR/${base}_mag.bin" -lm
  set +e
  local mout mrc2=0
  mout=$("$TMPDIR/${base}_mag.bin" 2>&1) || mrc2=$?
  set -e
  if [[ $mrc2 -ne 0 ]] && echo "$mout" | grep -qE $'ERR[\t ]*cap'; then
    pass "magic forge deny $label"
  else
    bad "magic forge $label out=$mout rc=$mrc2"
  fi
}

emit_run "$ROOT/fixtures/libfloor_tcp.oo" "lfn_tcp" "tcp-residual-ok" "tcp_connect" "oo_tcp_connect"
emit_run "$ROOT/fixtures/libfloor_tls.oo" "lfn_tls" "tls-residual-ok" "tls_connect" "oo_tls_connect"

# tcp_bind + bind_udp residual (inline)
cat >"$TMPDIR/lfn_udp.oo" <<'EOF'
pub fn main(net: &NetCap) {
    let a: Result[String, String] = tcp_bind(net, 0);
    let b: Result[String, String] = bind_udp(net, 0);
    if a.is_err() {
        if b.is_err() { println("udp-bind-residual-ok"); } else { println("udp-unexpected"); }
    } else { println("bind-unexpected"); }
}
EOF
set +e
"$OODAC_BIN" emit-c "$TMPDIR/lfn_udp.oo" >"$TMPDIR/lfn_udp.c" 2>"$TMPDIR/lfn_udp.err"
urc=$?
set -e
if [[ $urc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/lfn_udp.c" "$TMPDIR/lfn_udp.err" 2>/dev/null; then
  bad "emit-c tcp_bind/bind_udp"
  head -15 "$TMPDIR/lfn_udp.err" 2>/dev/null || true
elif ! grep -q 'oo_tcp_bind' "$TMPDIR/lfn_udp.c" || ! grep -q 'oo_bind_udp' "$TMPDIR/lfn_udp.c"; then
  bad "missing oo_tcp_bind/oo_bind_udp lowers"
else
  gcc "${RT[@]}" "$TMPDIR/lfn_udp.c" -o "$TMPDIR/lfn_udp.bin" 2>"$TMPDIR/lfn_udp.gcc" || {
    bad "gcc udp/bind"; head -10 "$TMPDIR/lfn_udp.gcc" || true
  }
  if [[ -x "$TMPDIR/lfn_udp.bin" ]]; then
    uout=$("$TMPDIR/lfn_udp.bin" 2>&1) || true
    if echo "$uout" | grep -q 'udp-bind-residual-ok'; then
      pass "runtime tcp_bind+bind_udp residual is_err"
    else
      bad "runtime udp/bind out=$uout"
    fi
  fi
fi

# runtime residual string honesty
if grep -q 'net residual: path A seal only' runtime/chs_rt_libfloor.c \
  && grep -q 'no full TCP/UDP/TLS product' runtime/chs_rt_libfloor.c; then
  pass "runtime residual err string present"
else
  bad "runtime residual err string missing"
fi

# path A honesty: no real OS sockets/TLS in libfloor residual
# (word-boundary so oo_tcp_connect / oo_bind_udp names do not false-positive)
if grep -nE '\bsocket\(|\bconnect\(|\bbind\(|\blisten\(|sys/socket|netinet/|openssl|SSL_|getaddrinfo' runtime/chs_rt_libfloor.c 2>/dev/null | head -5; then
  bad "libfloor residual claims real socket/TLS path"
else
  pass "no OS socket/TLS in libfloor residual"
fi

if [[ $fail -ne 0 ]]; then
  echo "libfloor_net_smoke: FAILED" >&2
  exit 1
fi
echo "libfloor_net_smoke: PASSED"
exit 0
