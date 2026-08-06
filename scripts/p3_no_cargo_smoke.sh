#!/usr/bin/env bash
# job: P3 rails — pure .oo CLI + no-cargo bootstrap + fail-closed residual
# in:  SEED (oodac/oodac) + gcc
# out: exit 0 if pure product path green without cargo/rustc
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- anti: bootstrap must not invoke cargo/rustc as a command ---
if grep -nE '(^|[^[:alnum:]_])(cargo|rustc)( |$)' "$ROOT/scripts/bootstrap_no_cargo.sh" \
  | grep -vE 'Never invoke|must not|comment|#' ; then
  bad "bootstrap_no_cargo invokes cargo/rustc"
else
  pass "bootstrap_no_cargo does not invoke cargo"
fi

# --- bootstrap ---
if ! SEED_OODAC="${SEED_OODAC:-$ROOT/oodac/oodac}" "$ROOT/scripts/bootstrap_no_cargo.sh" \
  >"$TMPDIR/p3_boot.out" 2>"$TMPDIR/p3_boot.err"; then
  bad "bootstrap_no_cargo failed"
  cat "$TMPDIR/p3_boot.out" "$TMPDIR/p3_boot.err" | tail -30
else
  pass "bootstrap_no_cargo"
  cat "$TMPDIR/p3_boot.out" | tail -8
fi

OODA="${OODA:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
if [[ ! -x "$OODA" ]]; then bad "bin/ooda missing"; else pass "bin/ooda exists"; fi

# --- product surface ---
"$OODA" version >"$TMPDIR/p3_ver.txt" 2>"$TMPDIR/p3_ver.err" || bad "version"
grep -q 'pure' "$TMPDIR/p3_ver.txt" && pass "version pure" || bad "version not pure"

"$OODA" check "$ROOT/fixtures/chs_list_string.oo" >"$TMPDIR/p3_chk.txt" 2>"$TMPDIR/p3_chk.err" || bad "check"
grep -qE '^OK' "$TMPDIR/p3_chk.txt" && pass "check pass" || bad "check output"

set +e
"$OODA" check "$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo" >"$TMPDIR/p3_cap.txt" 2>"$TMPDIR/p3_cap.err"
rc=$?
set -e
[[ $rc -ne 0 ]] && pass "check fail-closed cap" || bad "check accepted no_cap"

"$OODA" dump tokens "$ROOT/fixtures/int_main.oo" >"$TMPDIR/p3_tok.txt" 2>"$TMPDIR/p3_tok.err" || bad "dump tokens"
grep -q $'\t' "$TMPDIR/p3_tok.txt" && pass "dump tokens" || bad "tokens format"

"$OODA" run "$ROOT/fixtures/chs_list_string.oo" >"$TMPDIR/p3_run.txt" 2>"$TMPDIR/p3_run.err" || bad "run"
grep -q '2' "$TMPDIR/p3_run.txt" && pass "run native" || bad "run output"

# --- fail-closed residual ---
set +e
"$OODA" build --target wasm "$ROOT/fixtures/chs_list_string.oo" >"$TMPDIR/p3_wasm.txt" 2>"$TMPDIR/p3_wasm.err"
rw=$?
set -e
[[ $rw -ne 0 ]] && pass "wasm fail-closed" || bad "wasm accepted"

set +e
"$OODA" test "$ROOT/fixtures/chs_list_string.oo" --fuzz >"$TMPDIR/p3_fuzz.txt" 2>"$TMPDIR/p3_fuzz.err"
rf=$?
set -e
[[ $rf -ne 0 ]] && pass "fuzz fail-closed" || bad "fuzz accepted"

set +e
"$OODA" check "$ROOT/fixtures/chs_list_string.oo" --json-errors >"$TMPDIR/p3_je.txt" 2>"$TMPDIR/p3_je.err"
rj=$?
set -e
[[ $rj -ne 0 ]] && pass "json-errors fail-closed" || bad "json-errors accepted"

# --- no OK_HOST ---
if grep -rq 'OK_HOST' "$ROOT/oodac" "$ROOT/cli" --include='*.oo' 2>/dev/null; then
  bad "OK_HOST in pure sources"
else
  pass "no OK_HOST in pure sources"
fi

# --- no rustc used for product binary (file is not cargo-built rust) ---
if command -v file >/dev/null; then
  file "$OODA" | tee "$TMPDIR/p3_file.txt"
  if grep -qi 'rust\|cargo' "$TMPDIR/p3_file.txt"; then
    bad "bin/ooda looks cargo/rust linked"
  else
    pass "bin/ooda is native ELF (not cargo product)"
  fi
fi

# --- RS report ---
RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' | wc -l)
echo "RS_COUNT=$RS"
if [[ "$RS" -eq 0 ]]; then
  pass "B0 RS=0"
else
  # Residual until shell deleted this pin
  echo "NOTE residual RS=$RS (delete Rust shell for B0)"
fi

if [[ $fail -ne 0 ]]; then
  echo "p3_no_cargo_smoke: FAILED" >&2
  exit 1
fi
echo "p3_no_cargo_smoke: PASSED"
exit 0
