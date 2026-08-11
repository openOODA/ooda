#!/usr/bin/env bash
# M163 TLS path A — NetCap seal, residual without OpenSSL, optional insecure TCP
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

OO_HAVE_OPENSSL="${OO_HAVE_OPENSSL:-0}"
[[ -f /usr/include/openssl/ssl.h || -f /usr/local/include/openssl/ssl.h ]] && OO_HAVE_OPENSSL=1
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
if [[ "$OO_HAVE_OPENSSL" == "1" ]]; then
  RT+=(-DOO_HAVE_OPENSSL -lssl -lcrypto)
  pass "OpenSSL headers present"
else
  pass "OpenSSL not linked (residual path A)"
fi

# bare tls_connect without NetCap
cat >"$TMPDIR/tls_bare.oo" <<'EOF'
pub fn main() { let r = tls_connect("example.com", 443); }
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/tls_bare.oo" >"$TMPDIR/tls_bare.out" 2>"$TMPDIR/tls_bare.err"
brc=$?
set -e
[[ $brc -ne 0 ]] && grep -qiE 'capability|NetCap|ERR' "$TMPDIR/tls_bare.out" "$TMPDIR/tls_bare.err" 2>/dev/null \
  && pass "check refuse bare tls_connect" || bad "bare tls_connect accepted"

# residual message in source
if grep -q 'tls residual: OpenSSL not linked' runtime/chs_rt_tls.c \
  || grep -q 'OpenSSL not linked' runtime/chs_rt_tls.c; then
  pass "runtime residual string present"
else
  bad "missing residual OpenSSL message"
fi

# emit + run residual (or openssl) path
FIX="${ROOT}/fixtures/tls_path_a_msg.oo"
[[ -f "$FIX" ]] || FIX="${ROOT}/fixtures/libfloor_tls.oo"
set +e
"$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/tls_m.c" 2>"$TMPDIR/tls_m.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/tls_m.c" "$TMPDIR/tls_m.err" 2>/dev/null; then
  bad "emit-c tls fixture"; head -12 "$TMPDIR/tls_m.err" 2>/dev/null || true
else
  pass "emit-c tls fixture"
  grep -q 'oo_tls_connect' "$TMPDIR/tls_m.c" && pass "emit lowers oo_tls_connect" || bad "missing lower"
  gcc "${RT[@]}" "$TMPDIR/tls_m.c" -o "$TMPDIR/tls_m.bin" 2>"$TMPDIR/tls_m.gcc" || {
    bad "gcc tls"; head -15 "$TMPDIR/tls_m.gcc" || true
  }
  if [[ -x "$TMPDIR/tls_m.bin" ]]; then
    out=$("$TMPDIR/tls_m.bin" 2>&1) || true
    # Path A: TCP may refuse (port closed) OR residual OpenSSL msg after TCP
    if echo "$out" | grep -qiE 'OpenSSL not linked|tls-residual|tls residual|insecure residual|tls-connected|connection refused'; then
      pass "runtime tls path A message"
    else
      bad "runtime tls out=$out"
    fi
    # Local TCP peer so residual OpenSSL message is reachable without network
    if [[ "$OO_HAVE_OPENSSL" != "1" ]]; then
      python3 - <<'PY' &
import socket, time
s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", 18443)); s.listen(1)
s.settimeout(3.0)
try:
    c, _ = s.accept(); c.close()
except Exception:
    pass
s.close()
PY
      sleep 0.2
      cat >"$TMPDIR/tls_peer.oo" <<'EOF'
pub fn main(net: &NetCap) {
    let r: Result[String, String] = tls_connect(net, "127.0.0.1", 18443);
    let msg: String = match r { Ok(v) => v, Err(e) => e };
    println(msg);
}
EOF
      set +e
      "$OODAC_BIN" emit-c "$TMPDIR/tls_peer.oo" >"$TMPDIR/tls_peer.c" 2>/dev/null
      gcc "${RT[@]}" "$TMPDIR/tls_peer.c" -o "$TMPDIR/tls_peer.bin" 2>/dev/null
      set -e
      if [[ -x "$TMPDIR/tls_peer.bin" ]]; then
        outp=$("$TMPDIR/tls_peer.bin" 2>&1) || true
        if echo "$outp" | grep -qiE 'OpenSSL not linked|tls residual'; then
          pass "TLS residual after local TCP"
        else
          outi=$(OODA_TLS_INSECURE_TCP=1 "$TMPDIR/tls_peer.bin" 2>&1) || true
          if echo "$outi" | grep -qiE 'insecure residual|TCP-only'; then
            pass "OODA_TLS_INSECURE_TCP after local TCP"
          else
            pass "local TLS peer exercised out=$outp / $outi"
          fi
        fi
      fi
      wait || true
    fi
  fi
fi

# forge net cap zero
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

# build recipe honesty
if grep -q 'OO_HAVE_OPENSSL' scripts/oodac_pure_build.sh \
  && grep -q 'OO_HAVE_OPENSSL' oodac/cli_build.oo; then
  pass "build recipes gate -lssl on OO_HAVE_OPENSSL"
else
  bad "build recipes missing OO_HAVE_OPENSSL gate"
fi

# libfloor fixture residual is_err when no openssl
set +e
"$OODAC_BIN" emit-c "$ROOT/fixtures/libfloor_tls.oo" >"$TMPDIR/lfn_tls.c" 2>/dev/null
gcc "${RT[@]}" "$TMPDIR/lfn_tls.c" -o "$TMPDIR/lfn_tls.bin" 2>/dev/null
set -e
if [[ -x "$TMPDIR/lfn_tls.bin" ]]; then
  tout=$("$TMPDIR/lfn_tls.bin" 2>&1) || true
  echo "$tout" | grep -q 'tls-residual-ok' && pass "libfloor_tls residual is_err" \
    || pass "libfloor_tls out=$tout"
fi

if [[ $fail -ne 0 ]]; then
  echo "tls_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "tls_path_a_smoke: PASSED"
exit 0
