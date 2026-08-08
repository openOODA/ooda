#!/usr/bin/env bash
# job: product CLI pure-only dispatch (host frontend deleted)
# in:  release ooda + native oodac
# out: exit 0 if pure path + fail rails + anti FORCE_HOST host path
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: need pure $OODA (scripts/bootstrap_no_cargo.sh)" >&2
  exit 1
fi
if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

F="$ROOT/fixtures/int_main.oo"
CAP_FAIL="$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo"
LEX_FAIL="$ROOT/bootstrap/corpus/lex/fail/bad_char.oo"
BUILD_SRC="$ROOT/fixtures/chs_list_string.oo"

# --- pure dump tokens ≡ direct oodac ---
"$OODA" dump tokens "$F" >"$TMPDIR/prod_tok.txt"
"$OODAC" tokens "$F" >"$TMPDIR/oodac_tok.txt"
if diff -q "$TMPDIR/prod_tok.txt" "$TMPDIR/oodac_tok.txt" >/dev/null; then
  pass "product dump tokens == oodac"
else
  bad "product dump tokens diverge from oodac"
fi

# --- product check pure ---
"$OODA" check "$F" >"$TMPDIR/prod_chk.txt" 2>"$TMPDIR/prod_chk.err"
pc=$?
if [[ $pc -ne 0 ]]; then bad "product check pass exit=$pc"; else pass "product check pass"; fi
if grep -q 'openOODA check' "$TMPDIR/prod_chk.txt" 2>/dev/null; then
  bad "product check still uses host banner"
else
  pass "product check not host banner"
fi

# --- fail rails ---
set +e
"$OODA" check "$CAP_FAIL" >"$TMPDIR/cap.out" 2>"$TMPDIR/cap.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then bad "product check accepted no_cap_fetch"; else pass "product check fail-closed cap"; fi

set +e
"$OODA" dump tokens "$LEX_FAIL" >"$TMPDIR/lex.out" 2>"$TMPDIR/lex.err"
rl=$?
set -e
if [[ $rl -eq 0 ]]; then bad "product dump tokens accepted bad_char"; else pass "product dump fail-closed lex"; fi

# --- pure build-c ---
cp "$BUILD_SRC" "$TMPDIR/smoke.oo"
set +e
"$OODA" build --target c "$TMPDIR/smoke.oo" >"$TMPDIR/build.out" 2>"$TMPDIR/build.err"
brc=$?
set -e
if [[ -x "$TMPDIR/smoke" && $brc -eq 0 ]]; then
  set +e
  raw_smoke_out="$("$TMPDIR/smoke" 2>&1)"
  sm_rc=$?
  set -e
  if [[ $sm_rc -ne 0 ]]; then
    bad "product pure build-c execution failed exit=$sm_rc"
  else
    out=$(echo "$raw_smoke_out" | tr '\n' ',' | head -c 80)
    if echo "$out" | grep -q '2'; then
      pass "product pure build-c smoke ($out)"
    else
      bad "product pure build odd output: $out"
    fi
  fi
  rm -f "$TMPDIR/smoke"
else
  bad "product pure build-c missing binary"
  cat "$TMPDIR/build.out" "$TMPDIR/build.err" | head -20 || true
fi

# --- wasm fail-closed ---
set +e
"$OODA" build --target wasm "$F" >"$TMPDIR/wasm.out" 2>"$TMPDIR/wasm.err"
rw=$?
set -e
if [[ $rw -eq 0 ]]; then bad "wasm accepted"; else pass "wasm fail-closed"; fi

# --- pure native run ---
set +e
"$OODA" run "$BUILD_SRC" >"$TMPDIR/run.out" 2>"$TMPDIR/run.err"
rr=$?
set -e
if [[ $rr -ne 0 ]]; then
  bad "product pure run failed exit=$rr"
elif ! grep -q '2' "$TMPDIR/run.out"; then
  bad "product pure run missing expected output"
else
  pass "product pure run native"
fi

# --- test --fuzz native contract fuzzer ---
set +e
"$OODA" test "$BUILD_SRC" --fuzz >"$TMPDIR/fuzz.out" 2>"$TMPDIR/fuzz.err"
rz=$?
set -e
if [[ $rz -eq 0 ]] && grep -qE 'openOODA Fuzzer|passed' "$TMPDIR/fuzz.out"; then
  pass "test --fuzz native contract fuzzer active"
else
  bad "test --fuzz failed exit=$rz"
fi

# --- ooda test: real verify/assert_eq (P1 BUILD_OUT) ---
set +e
"$OODA" test "$ROOT/fixtures/verify_pass.oo" >"$TMPDIR/prod_test_ok.out" 2>"$TMPDIR/prod_test_ok.err"
tp=$?
set -e
if [[ $tp -ne 0 ]] || ! grep -q "OK verify" "$TMPDIR/prod_test_ok.out"; then
  bad "product test pass verify_pass exit=$tp"
else
  pass "product test verify_pass"
fi
set +e
"$OODA" test "$ROOT/fixtures/verify_fail.oo" >"$TMPDIR/prod_test_bad.out" 2>"$TMPDIR/prod_test_bad.err"
tf=$?
set -e
if [[ $tf -eq 0 ]]; then
  bad "product test accepted verify_fail"
else
  pass "product test fail-closed verify_fail"
fi
set +e
"$OODA" test "$ROOT/fixtures/verify_pass.oo" --fuzz >"$TMPDIR/prod_fuzz.out" 2>"$TMPDIR/prod_fuzz.err"
tzz=$?
set -e
if [[ $tzz -eq 0 ]] && grep -qE 'openOODA Fuzzer|passed' "$TMPDIR/prod_fuzz.out"; then
  pass "product test --fuzz active"
else
  bad "product test --fuzz failed exit=$tzz"
fi

# --- ooda patch replace_fn (P2 SAFE) ---
PATCH_SMOKE="$ROOT/scripts/patch_smoke.sh"
if [[ -x "$PATCH_SMOKE" ]]; then
  set +e
  "$PATCH_SMOKE" >"$TMPDIR/patch_smoke.out" 2>"$TMPDIR/patch_smoke.err"
  ps=$?
  set -e
  if [[ $ps -ne 0 ]]; then
    bad "patch_smoke exit=$ps"
    head -20 "$TMPDIR/patch_smoke.err" 2>/dev/null || true
  else
    pass "patch_smoke"
  fi
else
  bad "missing patch_smoke.sh"
fi

# --- problem-hunt honesty rails (differential / mutation / contracts) ---
if [[ -x "$ROOT/scripts/problem_hunt_smoke.sh" ]]; then
  set +e
  "$ROOT/scripts/problem_hunt_smoke.sh" >"$TMPDIR/prod_ph.out" 2>"$TMPDIR/prod_ph.err"
  ph=$?
  set -e
  if [[ $ph -ne 0 ]]; then
    bad "problem_hunt_smoke"
    tail -15 "$TMPDIR/prod_ph.err" "$TMPDIR/prod_ph.out" 2>/dev/null || true
  else
    pass "problem_hunt_smoke"
  fi
fi

# --- host modules + Rust shell gone (B0) ---
if [[ -d "$ROOT/src" ]]; then
  bad "src/ still present"
else
  pass "src/ deleted (no Rust shell)"
fi
if [[ -f "$ROOT/Cargo.toml" ]]; then
  bad "Cargo.toml still present"
else
  pass "Cargo.toml deleted (no Cargo product)"
fi

# --- no OK_HOST in pure sources ---
if grep -rI --include='*.oo' 'OK_HOST' "$ROOT/oodac" "$ROOT/cli" 2>/dev/null | grep -v '//' | grep -q OK_HOST; then
  bad "OK_HOST still in pure sources"
else
  pass "no OK_HOST in pure sources"
fi

RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' | wc -l)
echo "RS_COUNT=$RS"
if [[ "$RS" -eq 0 ]]; then
  pass "B0 RS_COUNT=0"
else
  bad "RS_COUNT=$RS (want 0)"
fi

# Product binary must be pure (bin/ooda), not cargo target
if [[ "$OODA" == *target/release* ]]; then
  bad "OODA still points at cargo target"
else
  pass "OODA=$OODA pure path"
fi

if [[ $fail -ne 0 ]]; then
  echo "product_pure_dispatch_smoke: FAILED" >&2
  exit 1
fi
echo "product_pure_dispatch_smoke: PASSED"
exit 0
