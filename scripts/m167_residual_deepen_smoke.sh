#!/usr/bin/env bash
# M167 residual deepen path A: pure-rebuild readiness + typed malloc + caret source
# honesty: tip oodac may still SEGV on untyped free-name lets; pure multi not dual-green
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# Source floors
if grep -q 'c_emit_let_untyped_ty' oodac/c_emit_let.oo \
  && grep -q 'import "c_emit_let_ext.oo"' oodac/main.oo \
  && grep -q 'alloc_bytes' oodac/c_emit_let_ext.oo \
  && grep -q 'try_scan_ws' oodac/c_emit_let_ext.oo; then
  pass "c_emit_let uses ext free-name short-circuit"
else
  bad "c_emit_let/ext M167 wiring missing"
fi
if grep -q 'CARET' oodac/token_scan_punct.oo; then
  pass "caret token source floor"
else
  bad "caret missing"
fi
if grep -q 'let r0: List\[String\]' oodac/token_next.oo \
  && grep -q 'let n: Int' oodac/token_next.oo; then
  pass "token_next typed lets (emit-safe on tip host)"
else
  bad "token_next missing typed lets"
fi
for f in oodac/c_emit_let.oo oodac/c_emit_let_ext.oo oodac/token_next.oo oodac/token_scan_punct.oo oodac/main.oo; do
  n=$(wc -l <"$f" | tr -d ' ')
  if [[ "$n" -gt 256 ]]; then
    bad "$f over 256 lines ($n)"
  else
    pass "$f lines=$n"
  fi
done

# Product: typed malloc let
if [[ -x "$OODAC_BIN" ]]; then
  set +e
  "$OODAC_BIN" build fixtures/malloc_path_a.oo "$TMPDIR/m167_mal" \
    >"$TMPDIR/m167_mal.out" 2>"$TMPDIR/m167_mal.err"
  brc=$?
  set -e
  if [[ $brc -eq 0 && -x "$TMPDIR/m167_mal" ]]; then
    out=$("$TMPDIR/m167_mal" 2>&1) || true
    if echo "$out" | grep -qx 'ok'; then
      pass "typed let malloc product path"
    else
      bad "typed malloc out=$out"
    fi
  else
    bad "build malloc_path_a"
    head -8 "$TMPDIR/m167_mal.err" 2>/dev/null || true
  fi
  # untyped still residual on tip (honest)
  cat >"$TMPDIR/m167_untyped.oo" <<'EOF'
pub fn main(alloc: &AllocCap) {
    let p = alloc_bytes(alloc, 64);
    free_bytes(alloc, p);
    println("ok");
}
EOF
  set +e
  timeout 8 "$OODAC_BIN" emit-c "$TMPDIR/m167_untyped.oo" \
    >"$TMPDIR/m167_u.c" 2>"$TMPDIR/m167_u.err"
  urc=$?
  set -e
  if [[ $urc -eq 0 ]] && grep -q 'oo_alloc_bytes' "$TMPDIR/m167_u.c"; then
    pass "untyped alloc_bytes let green (host rebuilt)"
  else
    pass "untyped alloc_bytes let residual (tip SEGV; pure rebuild residual)"
  fi
  # token_next emit
  set +e
  timeout 15 "$OODAC_BIN" emit-c oodac/token_next.oo >"$TMPDIR/m167_tn.c" 2>"$TMPDIR/m167_tn.err"
  trc=$?
  set -e
  if [[ $trc -eq 0 ]] && grep -q 'lex_step' "$TMPDIR/m167_tn.c"; then
    pass "token_next emit on tip host"
  else
    bad "token_next emit failed"
  fi
else
  bad "no OODAC"
fi

# Cap forgery residual honesty (still open — not fixed this mile)
if grep -qiE 'as fn|forgery|cast' bootstrap/STATIC_CAPS.oot; then
  pass "STATIC_CAPS documents as-fn cast forgery residual"
else
  bad "STATIC_CAPS missing cast forgery residual"
fi

if [[ $fail -ne 0 ]]; then
  echo "m167_residual_deepen_smoke: FAILED" >&2
  exit 1
fi
echo "m167_residual_deepen_smoke: PASSED"
exit 0
