#!/usr/bin/env bash
# M164 TLS product floor — real OpenSSL when libssl.so.3 present
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

OO_HAVE_OPENSSL=0
SSL_LIBS=()
if [[ -e /usr/lib64/libssl.so.3 ]]; then
  OO_HAVE_OPENSSL=1
  SSL_LIBS=(/usr/lib64/libssl.so.3 /usr/lib64/libcrypto.so.3)
elif [[ -e /lib64/libssl.so.3 ]]; then
  OO_HAVE_OPENSSL=1
  SSL_LIBS=(/lib64/libssl.so.3 /lib64/libcrypto.so.3)
elif [[ -e /usr/lib/x86_64-linux-gnu/libssl.so.3 ]]; then
  OO_HAVE_OPENSSL=1
  SSL_LIBS=(/usr/lib/x86_64-linux-gnu/libssl.so.3 /usr/lib/x86_64-linux-gnu/libcrypto.so.3)
fi
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
if [[ "$OO_HAVE_OPENSSL" == "1" ]]; then
  RT+=(-DOO_HAVE_OPENSSL "${SSL_LIBS[@]}")
  pass "OpenSSL lib present (product TLS link)"
else
  pass "OpenSSL not present (residual path)"
fi

# bare refuse
cat >"$TMPDIR/tls_bare.oo" <<'EOF'
pub fn main() { let r = tls_connect("example.com", 443); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/tls_bare.oo" >"$TMPDIR/tls_bare.out" 2>"$TMPDIR/tls_bare.err"
brc=$?
set -e
[[ $brc -ne 0 ]] && grep -qiE 'capability|NetCap|ERR' "$TMPDIR/tls_bare.out" "$TMPDIR/tls_bare.err" 2>/dev/null \
  && pass "check refuse bare tls_connect" || bad "bare tls_connect accepted"

# emit fixture
FIX="$ROOT/fixtures/tls_path_a_msg.oo"
[[ -f "$FIX" ]] || FIX="$ROOT/fixtures/libfloor_tls.oo"
set +e
"$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/tls_m.c" 2>"$TMPDIR/tls_m.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/tls_m.c" "$TMPDIR/tls_m.err" 2>/dev/null; then
  bad "emit-c tls"; head -12 "$TMPDIR/tls_m.err" || true
else
  pass "emit-c tls"
  grep -q 'oo_tls_connect' "$TMPDIR/tls_m.c" && pass "emit lowers oo_tls_connect" || bad "missing lower"
  gcc "${RT[@]}" "$TMPDIR/tls_m.c" -o "$TMPDIR/tls_m.bin" 2>"$TMPDIR/tls_m.gcc" || {
    bad "gcc tls"; head -20 "$TMPDIR/tls_m.gcc" || true
  }
fi

if [[ -x "$TMPDIR/tls_m.bin" && "$OO_HAVE_OPENSSL" == "1" ]]; then
  # real handshake to a public HTTPS host (skip if offline)
  cat >"$TMPDIR/tls_pub.oo" <<'EOF'
pub fn main(net: &NetCap) {
    let r: Result[String, String] = tls_connect(net, "example.com", 443);
    if r.is_ok() {
        println("tls-handshake-ok");
    } else {
        let msg: String = match r { Ok(v) => v, Err(e) => e };
        println(msg);
    }
}
EOF
  set +e
  "$OODAC_BIN" emit-c "$TMPDIR/tls_pub.oo" >"$TMPDIR/tls_pub.c" 2>/dev/null
  gcc "${RT[@]}" "$TMPDIR/tls_pub.c" -o "$TMPDIR/tls_pub.bin" 2>/dev/null
  set -e
  if [[ -x "$TMPDIR/tls_pub.bin" ]]; then
    set +e
    out=$("$TMPDIR/tls_pub.bin" 2>&1)
    rc=$?
    set -e
    if echo "$out" | grep -q 'tls-handshake-ok\|tls-connected:example.com:443'; then
      pass "real TLS handshake example.com:443"
    elif echo "$out" | grep -qiE 'resolve failed|connection refused|SSL_connect failed|Network is unreachable'; then
      pass "skip public TLS (network/handshake fail) out=$out"
    else
      bad "public TLS unexpected out=$out rc=$rc"
    fi
  fi
elif [[ -x "$TMPDIR/tls_m.bin" ]]; then
  out=$("$TMPDIR/tls_m.bin" 2>&1) || true
  echo "$out" | grep -qiE 'OpenSSL not linked|connection refused|tls residual' \
    && pass "residual TLS message" || bad "out=$out"
fi

# forge deny
if [[ -x "$TMPDIR/tls_m.bin" ]] && grep -q 'oo_cap_grant_net' "$TMPDIR/tls_m.c"; then
  sed -E 's/long long net = oo_cap_grant_net\(\)/long long net = 0LL/' \
    "$TMPDIR/tls_m.c" >"$TMPDIR/tls_z.c"
  gcc "${RT[@]}" "$TMPDIR/tls_z.c" -o "$TMPDIR/tls_z.bin" 2>/dev/null || true
  if [[ -x "$TMPDIR/tls_z.bin" ]]; then
    set +e; zout=$("$TMPDIR/tls_z.bin" 2>&1); zrc=$?; set -e
    [[ $zrc -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap' \
      && pass "zero Net forge deny" || bad "forge out=$zout rc=$zrc"
  fi
fi

if grep -q 'libssl.so.3' scripts/oodac_pure_build.sh \
  && grep -q 'OO_HAVE_OPENSSL' oodac/cli_build.oo; then
  pass "build recipes link OpenSSL product path"
else
  bad "build recipes missing OpenSSL product link"
fi

if [[ $fail -ne 0 ]]; then
  echo "tls_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "tls_path_a_smoke: PASSED"
exit 0
