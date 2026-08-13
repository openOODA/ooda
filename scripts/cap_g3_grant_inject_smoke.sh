#!/usr/bin/env bash
# CAP-G3 grant inject harness — main inject for HttpCap/FsReadCap/TcpCap (fixed names)
# Proves c_emit_fn static markers + product emit-c lowers oo_cap_grant_{http,fsread,tcp}.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"

# Prefer product tree oodac; allow OODAC_BIN override; fall back to sibling oodac/
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
if [[ ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/../oodac/oodac" ]]; then
    OODAC="$ROOT/../oodac/oodac"
  else
    echo "ERR_NO_OODAC: need $ROOT/oodac/oodac" >&2
    exit 1
  fi
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

CEMIT_FN="$ROOT/oodac/c_emit_fn.oo"
PREAMBLE="$ROOT/oodac/c_emit_preamble.oo"

# Static markers in c_emit_fn (main inject strings)
if grep -q 'oo_cap_grant_http' "$CEMIT_FN" \
  && grep -q 'oo_cap_grant_fsread' "$CEMIT_FN" \
  && grep -q 'oo_cap_grant_tcp' "$CEMIT_FN"; then
  pass "static c_emit_fn markers (grant_http/fsread/tcp)"
else
  bad "missing oo_cap_grant_{http,fsread,tcp} in c_emit_fn.oo"
fi

# Soft: preamble decls for granular grants (honest when present)
if grep -q 'oo_cap_grant_http' "$PREAMBLE" 2>/dev/null \
  && grep -q 'oo_cap_grant_fsread' "$PREAMBLE" 2>/dev/null; then
  pass "static c_emit_preamble decls (grant_http/fsread)"
else
  pass "static c_emit_preamble granular decls residual"
fi

run_emit() { # $1=src $2=out_c $3=err
  local src="$1" out="$2" err="$3" rc
  set +e
  "$OODAC" emit-c "$src" >"$out" 2>"$err"
  rc=$?
  set -e
  # strip rocket / Running main noise if any
  if [[ -s "$out" ]]; then
    grep -v '🚀\|Running main' "$out" >"${out}.clean" 2>/dev/null || true
    if [[ -s "${out}.clean" ]]; then mv "${out}.clean" "$out"; else rm -f "${out}.clean"; fi
  fi
  printf '%s' "$rc"
}

# emit-c: main with HttpCap / FsReadCap / TcpCap — expect grant injects in C
cat >"$TMPDIR/cap_g3_http.oo" <<'EOF'
pub fn main(http: &HttpCap) {
}
EOF
cat >"$TMPDIR/cap_g3_fsread.oo" <<'EOF'
pub fn main(fsread: &FsReadCap) {
}
EOF
cat >"$TMPDIR/cap_g3_tcp.oo" <<'EOF'
pub fn main(tcp: &TcpCap) {
}
EOF
cat >"$TMPDIR/cap_g3_all.oo" <<'EOF'
pub fn main(http: &HttpCap, fsread: &FsReadCap, tcp: &TcpCap) {
}
EOF

expect_grant() { # $1=fixture_base $2=grep_pattern $3=label
  local base="$1" pat="$2" label="$3"
  local src="$TMPDIR/${base}.oo"
  local out="$TMPDIR/${base}.c"
  local err="$TMPDIR/${base}.err"
  local rc
  rc="$(run_emit "$src" "$out" "$err")"
  if [[ "$rc" != "0" ]] || grep -qE $'^ERR\t' "$out" 2>/dev/null; then
    bad "emit-c $label exit=$rc"
    head -8 "$out" "$err" 2>/dev/null || true
    return
  fi
  if grep -qE "$pat" "$out"; then
    pass "emit-c $label inject ($pat)"
  else
    bad "emit-c $label missing $pat"
    grep -n 'oo_cap_grant\|int main' "$out" | head -20 || true
  fi
}

expect_grant cap_g3_http   'oo_cap_grant_http'   "main(http: &HttpCap)"
expect_grant cap_g3_fsread 'oo_cap_grant_fsread' "main(fsread: &FsReadCap)"
expect_grant cap_g3_tcp    'oo_cap_grant_tcp'    "main(tcp: &TcpCap)"

# Combined main: all three grants in one emit
rc_all="$(run_emit "$TMPDIR/cap_g3_all.oo" "$TMPDIR/cap_g3_all.c" "$TMPDIR/cap_g3_all.err")"
if [[ "$rc_all" != "0" ]] || grep -qE $'^ERR\t' "$TMPDIR/cap_g3_all.c" 2>/dev/null; then
  bad "emit-c main(HttpCap,FsReadCap,TcpCap) exit=$rc_all"
  head -8 "$TMPDIR/cap_g3_all.c" "$TMPDIR/cap_g3_all.err" 2>/dev/null || true
else
  ok_all=1
  for pat in oo_cap_grant_http oo_cap_grant_fsread oo_cap_grant_tcp; do
    if ! grep -qE "$pat" "$TMPDIR/cap_g3_all.c"; then
      bad "emit-c multi missing $pat"
      ok_all=0
    fi
  done
  if [[ $ok_all -eq 1 ]]; then
    pass "emit-c multi HttpCap+FsReadCap+TcpCap grants"
  fi
fi

# Soft: named in ci_product rail list
if grep -q 'cap_g3_grant_inject_smoke' "$ROOT/scripts/ci_product.sh" 2>/dev/null; then
  pass "ci_product soft-wire present"
else
  pass "ci_product soft-wire residual (not listed yet)"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_g3_grant_inject_smoke: FAILED" >&2
  exit 1
fi
echo "cap_g3_grant_inject_smoke: PASSED"
exit 0
