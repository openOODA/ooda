#!/usr/bin/env bash
# M166 path A — list_get product + index_get alias + xs[i] LBRACKET sugar
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
FIX="$ROOT/fixtures/list_index.oo"

# Source floor: index sugar + free alias
if grep -q 'c_emit_index_get' oodac/c_emit_call.oo \
  && grep -q 'LBRACKET' oodac/c_emit_lower.oo \
  && grep -q 'index_get' oodac/tc_names.oo; then
  pass "emit/check index sugar + index_get known"
else
  bad "missing index path A wiring"
fi
if grep -q 'index_get=2' oodac/tc_call_arity.oo; then
  pass "tc_call_arity index_get=2"
else
  bad "arity seed missing index_get"
fi

[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: $OODAC" >&2; exit 1; }

# list_get-only fixture always product (no sugar needed)
cat >"$TMPDIR/li_get.oo" <<'EOF'
pub fn main() {
    let mut xs: List[Int] = list_new();
    xs = list_push(xs, 42);
    println(list_get(xs, 0));
}
EOF
set +e
"$OODAC" emit-c "$TMPDIR/li_get.oo" >"$TMPDIR/li_get.c" 2>"$TMPDIR/li_get.err"
erc=$?
set -e
if [[ $erc -eq 0 ]] && grep -qE 'oo_ilist_get' "$TMPDIR/li_get.c"; then
  pass "list_get product lowers oo_ilist_get"
  gcc "${RT[@]}" "$TMPDIR/li_get.c" -o "$TMPDIR/li_get.bin" 2>/dev/null || true
  if [[ -x "$TMPDIR/li_get.bin" ]]; then
    out=$("$TMPDIR/li_get.bin" 2>&1) || true
    if echo "$out" | grep -qx '42'; then
      pass "list_get runtime 42"
    else
      bad "list_get runtime out=$out"
    fi
  fi
else
  bad "list_get emit failed"
fi

# Full fixture (sugar needs rebuilt oodac with LBRACKET lower)
set +e
"$OODAC" check "$FIX" >"$TMPDIR/li_ck.out" 2>"$TMPDIR/li_ck.err"
ckrc=$?
set -e
if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/li_ck.out"; then
  pass "check list_index.oo"
else
  # sugar may confuse check if not rebuilt — still soft-pass source floor above
  bad "check list_index rc=$ckrc (rebuild oodac if LBRACKET residual)"
  head -8 "$TMPDIR/li_ck.out" "$TMPDIR/li_ck.err" 2>/dev/null || true
fi

set +e
"$OODAC" emit-c "$FIX" >"$TMPDIR/li.c" 2>"$TMPDIR/li.err"
erc2=$?
set -e
if [[ $erc2 -eq 0 ]] && ! grep -qE $'^ERR\t' "$TMPDIR/li.c" "$TMPDIR/li.err" 2>/dev/null; then
  if grep -qE 'oo_ilist_get' "$TMPDIR/li.c"; then
    pass "emit list_index lowers oo_ilist_get"
  else
    bad "emit missing oo_ilist_get"
  fi
  gcc "${RT[@]}" "$TMPDIR/li.c" -o "$TMPDIR/li.bin" 2>"$TMPDIR/li.gcc" || {
    bad "gcc list_index"; head -10 "$TMPDIR/li.gcc" || true
  }
  if [[ -x "$TMPDIR/li.bin" ]]; then
    out=$("$TMPDIR/li.bin" 2>&1) || true
    # expect six lines: 10 20 30 10 20 30
    if echo "$out" | tr '\n' ' ' | grep -qE '10.*20.*30.*10.*20.*30'; then
      pass "runtime list_index product"
    else
      bad "runtime list_index out=$out"
    fi
  fi
else
  bad "emit-c list_index (need oodac rebuild for xs[i] sugar)"
  head -12 "$TMPDIR/li.err" "$TMPDIR/li.c" 2>/dev/null || true
fi

if [[ $fail -ne 0 ]]; then
  echo "list_index_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "list_index_path_a_smoke: PASSED"
exit 0
