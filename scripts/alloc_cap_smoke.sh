#!/usr/bin/env bash
# M17 AllocCap rails — process-local seal only (not OS rlimit / heap isolation)
# check pass/fail + product deny + runtime grant + forge zero/classic deny
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
OODA="${OODA:-$ROOT/bin/ooda}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c")
deny_forge() { # $1=c $2=sed_expr $3=label
  local fc="${1%.c}_forge.c" fb="${1%.c}_forge.bin"
  sed -E "$2" "$1" >"$fc"
  gcc "${RT[@]}" "$fc" -o "$fb" -lm
  set +e; local o rc=0; o=$("$fb" 2>&1) || rc=$?; set -e
  if [[ $rc -ne 0 ]] && echo "$o" | grep -qE $'ERR[\t ]*cap'; then pass "$3"
  else bad "$3 out=$o rc=$rc"; fi
}
PASS="$ROOT/bootstrap/corpus/check/pass/ok_alloc_bytes.oo"
FAIL="$ROOT/bootstrap/corpus/check/fail/no_cap_alloc_bytes.oo"
set +e; "$OODAC" check "$PASS" >"$TMPDIR/ac_pass.out" 2>"$TMPDIR/ac_pass.err"; prc=$?; set -e
if [[ $prc -ne 0 ]] || ! grep -qE '^OK' "$TMPDIR/ac_pass.out"; then
  bad "check pass ok_alloc_bytes exit=$prc"
  cat "$TMPDIR/ac_pass.out" "$TMPDIR/ac_pass.err" | head -5 || true
else pass "check allow ok_alloc_bytes"; fi
set +e; "$OODAC" check "$FAIL" >"$TMPDIR/ac_fail.out" 2>"$TMPDIR/ac_fail.err"; frc=$?; set -e
if [[ $frc -eq 0 ]]; then bad "check accepted no_cap_alloc_bytes"
elif ! grep -qE 'capability|ERR' "$TMPDIR/ac_fail.out" "$TMPDIR/ac_fail.err" 2>/dev/null; then
  bad "check fail no_cap_alloc_bytes missing ERR (exit=$frc)"
else pass "check deny no_cap_alloc_bytes"; fi
# Product static deny
if [[ -x "$OODA" ]]; then
  set +e
  "$OODA" check "$FAIL" >"$TMPDIR/ac_prod.out" 2>"$TMPDIR/ac_prod.err"
  prc=$?
  set -e
  if [[ $prc -eq 0 ]]; then bad "product check accepted no_cap_alloc_bytes"
  else pass "product check deny no_cap_alloc_bytes"; fi
fi
# emit without cap IDENT must fail-closed
cat >"$TMPDIR/ac_bad.oo" <<'EOF'
pub fn main(alloc: &AllocCap) { let p = alloc_bytes(1, 16); }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/ac_bad.oo" >"$TMPDIR/ac_bad.c" 2>"$TMPDIR/ac_bad.err"; brc=$?; set -e
if grep -qE $'^ERR\tc_emit\talloc_bytes requires' "$TMPDIR/ac_bad.c" "$TMPDIR/ac_bad.err" 2>/dev/null; then
  pass "emit alloc_bytes without AllocCap arg fail-closed"
elif [[ $brc -ne 0 ]]; then pass "emit alloc_bytes without AllocCap arg non-zero exit"
else bad "emit lowered alloc_bytes without AllocCap arg (must fail-closed)"; fi
# Runtime pass with grant inject name `alloc` (c_emit_fn)
cat >"$TMPDIR/ac_rt.oo" <<'EOF'
pub fn main(alloc: &AllocCap) {
    let p = alloc_bytes(alloc, 64);
    free_bytes(alloc, p);
    println("alloc-ok");
}
EOF
set +e; "$OODAC" emit-c "$TMPDIR/ac_rt.oo" >"$TMPDIR/ac_rt.c" 2>"$TMPDIR/ac_rt.err"; rrc=$?; set -e
if [[ $rrc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/ac_rt.c"; then
  bad "emit alloc_bytes"
  head -5 "$TMPDIR/ac_rt.err" 2>/dev/null || true
elif ! grep -qE 'oo_alloc_bytes\(alloc' "$TMPDIR/ac_rt.c"; then
  bad "emit alloc_bytes must pass alloc cap first"
else
  gcc "${RT[@]}" "$TMPDIR/ac_rt.c" -o "$TMPDIR/ac_rt.bin" -lm
  out=$("$TMPDIR/ac_rt.bin" 2>&1) || true
  if echo "$out" | grep -q 'alloc-ok'; then pass "runtime Alloc alloc_bytes"
  else bad "runtime Alloc out=$out"; fi
  # classic magic OOAL = 0x4F4F414C
  deny_forge "$TMPDIR/ac_rt.c" \
    's/long long alloc = oo_cap_grant_alloc\(\)/long long alloc = 0LL/' \
    "runtime Alloc forged cap deny (zero)"
  deny_forge "$TMPDIR/ac_rt.c" \
    's/long long alloc = oo_cap_grant_alloc\(\)/long long alloc = 0x4F4F414CLL/' \
    "runtime Alloc classic magic-int deny"
fi
if [[ $fail -ne 0 ]]; then echo "alloc_cap_smoke: FAILED" >&2; exit 1; fi
echo "alloc_cap_smoke: PASSED"
exit 0
