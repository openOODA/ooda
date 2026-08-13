#!/usr/bin/env bash
# M169 path A — cap forgery posture (not full object-caps)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

if grep -q 'M169 path A — cast forgery' bootstrap/STATIC_CAPS.oot \
  && grep -qiE 'Not a product surface|as fn' bootstrap/STATIC_CAPS.oot; then
  pass "STATIC_CAPS M169 cast forgery posture"
else
  bad "STATIC_CAPS missing M169 cast section"
fi

if [[ ! -x "$OODAC_BIN" ]]; then
  bad "no OODAC"; exit 1
fi

# bare sealed free name refused at check
cat >"$TMPDIR/forge_bare.oo" <<'EOF'
pub fn main() {
    let r = dlopen("libc.so.6");
    println(1);
}
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/forge_bare.oo" >"$TMPDIR/fb.out" 2>"$TMPDIR/fb.err"
brc=$?
set -e
if [[ $brc -ne 0 ]] && grep -qiE 'capability|UnsafeFFICap|default-deny|sealed' "$TMPDIR/fb.out" "$TMPDIR/fb.err" 2>/dev/null; then
  pass "check refuse bare dlopen (E_CAP)"
else
  bad "bare dlopen not refused rc=$brc"
  head -5 "$TMPDIR/fb.out" "$TMPDIR/fb.err" || true
fi

# with cap: check OK (product path A)
FIX=fixtures/cap_forge_cast_residual.oo
if [[ -f "$FIX" ]]; then
  set +e
  "$OODAC_BIN" check "$FIX" >"$TMPDIR/fc.out" 2>"$TMPDIR/fc.err"
  crc=$?
  set -e
  if [[ $crc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/fc.out"; then
    pass "check dlopen with &UnsafeFFICap"
  else
    bad "check with cap failed"; head -8 "$TMPDIR/fc.out" "$TMPDIR/fc.err" || true
  fi
else
  bad "missing fixture cap_forge_cast_residual.oo"
fi

# `as` reserved — not usable as cast keyword soft-pass
cat >"$TMPDIR/as_cast.oo" <<'EOF'
pub fn main() {
    let x = dlopen as fn(&UnsafeFFICap, String) -> Int;
    println(x);
}
EOF
set +e
"$OODAC_BIN" check "$TMPDIR/as_cast.oo" >"$TMPDIR/as.out" 2>"$TMPDIR/as.err"
arc=$?
set -e
if [[ $arc -ne 0 ]]; then
  pass "as-fn cast does not soft-pass check (rc=$arc)"
else
  bad "as-fn cast soft-passed check"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_forge_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "cap_forge_path_a_smoke: PASSED"
exit 0
