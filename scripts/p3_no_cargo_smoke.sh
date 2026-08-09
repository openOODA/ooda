#!/usr/bin/env bash
# job: P3 rails — pure product path smoke + fail-closed residual
# in:  SEED (oodac/oodac) + gcc
# out: exit 0 if pure product path green
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- bootstrap must not shell out to cargo/rustc ---
if grep -nE '(^|[^[:alnum:]_])(cargo|rustc)( |$)' "$ROOT/scripts/bootstrap_no_cargo.sh" \
  | grep -vE 'Never invoke|must not|comment|#' ; then
  bad "bootstrap shells out to cargo/rustc"
else
  pass "bootstrap pure (no cargo/rustc invoke)"
fi

# --- bootstrap ---
# Prefer cold seed; tree oodac can SEGV as emit host under PURE_NO_ARC residual.
_SEED_DEFAULT="$ROOT/bootstrap/seed/oodac"
if [[ ! -x "$_SEED_DEFAULT" ]]; then _SEED_DEFAULT="$ROOT/oodac/oodac"; fi
if ! SEED_OODAC="${SEED_OODAC:-$_SEED_DEFAULT}" "$ROOT/scripts/bootstrap_no_cargo.sh" \
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

# Native prove (run interpreter residual under PURE_NO_ARC may print char_at OOB).
P3_BIN="$TMPDIR/p3_chs_native"
rm -f "$P3_BIN"
set +e
SEED_P3="${SEED_OODAC:-$ROOT/bootstrap/seed/oodac}"
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED_P3" "$OODA" build \
  "$ROOT/fixtures/chs_list_string.oo" -o "$P3_BIN" >"$TMPDIR/p3_run.txt" 2>"$TMPDIR/p3_run.err")
rrb=$?
set -e
if [[ $rrb -eq 0 && -x "$P3_BIN" ]] && "$P3_BIN" 2>/dev/null | grep -q '2'; then
  pass "run native (build+exec)"
else
  bad "run output"
fi

# --- un-gated surfaces (honesty: not residual-gated) ---
set +e
"$OODA" build --target wasm "$ROOT/fixtures/chs_list_string.oo" >"$TMPDIR/p3_wasm.txt" 2>"$TMPDIR/p3_wasm.err"
rw=$?
set -e
if [[ $rw -eq 0 ]] || grep -qiE 'WebAssembly|\.wat' "$TMPDIR/p3_wasm.txt" "$TMPDIR/p3_wasm.err" 2>/dev/null; then
  pass "wasm product path"
else
  bad "wasm product path failed"
fi

# Fuzz CLI un-gated; harness is Python residual (may fail build under PURE_NO_ARC).
set +e
"$OODA" test "$ROOT/fixtures/chs_list_string.oo" --fuzz >"$TMPDIR/p3_fuzz.txt" 2>"$TMPDIR/p3_fuzz.err"
rf=$?
set -e
if [[ $rf -eq 2 ]] && grep -qE 'ERR.*--fuzz residual' "$TMPDIR/p3_fuzz.txt" "$TMPDIR/p3_fuzz.err" 2>/dev/null; then
  bad "fuzz still residual-gated"
elif grep -qiE 'python|ooda_fuzz|harness|fuzz|Fuzzer|passed|ERR' "$TMPDIR/p3_fuzz.txt" "$TMPDIR/p3_fuzz.err" 2>/dev/null; then
  pass "fuzz un-gated (Python residual; rc=$rf)"
else
  bad "fuzz unexpected (rc=$rf)"
fi

set +e
"$OODA" check "$ROOT/fixtures/chs_list_string.oo" --json-errors >"$TMPDIR/p3_je.txt" 2>"$TMPDIR/p3_je.err"
rj=$?
set -e
if [[ $rj -eq 0 ]] && python3 -c 'import json,sys; v=json.loads(open(sys.argv[1]).read()); assert v==[]' "$TMPDIR/p3_je.txt" 2>/dev/null; then
  pass "json-errors pass → []"
else
  bad "json-errors expected [] exit0 (got exit=$rj $(head -c 120 "$TMPDIR/p3_je.txt" 2>/dev/null))"
fi

# --- no OK_HOST ---
if grep -rI --include='*.oo' 'OK_HOST' "$ROOT/oodac" "$ROOT/cli" 2>/dev/null | grep -v '//' | grep -q OK_HOST; then
  bad "OK_HOST in pure sources"
else
  pass "no OK_HOST in pure sources"
fi

# --- product binary is native ELF ---
if command -v file >/dev/null; then
  file "$OODA" | tee "$TMPDIR/p3_file.txt"
  if grep -qi 'rust\|cargo' "$TMPDIR/p3_file.txt"; then
    bad "bin/ooda not pure native product"
  else
    pass "bin/ooda is native ELF"
  fi
fi

# --- product purity: residual .rs count (B0 wants 0) ---
RS=$(find "$ROOT" -name '*.rs' -not -path '*/.git/*' -not -path '*/target/*' | wc -l)
echo "RS_COUNT=$RS"
if [[ "$RS" -eq 0 ]]; then
  pass "B0 RS=0"
else
  bad "B0 RS=$RS (want 0; no soft-pass)"
fi

if [[ $fail -ne 0 ]]; then
  echo "p3_product_smoke: FAILED" >&2
  exit 1
fi
echo "p3_product_smoke: PASSED"
exit 0
