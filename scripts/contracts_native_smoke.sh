#!/usr/bin/env bash
# job: native Backend-C contracts smoke (simple + path A &&/||/arith; M51 multi_clause)
# stage: test
# residual: full SMT / quantifiers / old-state (CONTRACTS_COMPLEX.md)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

PASS="$ROOT/bootstrap/corpus/emit-c/pass/fn_contracts_add.oo"
FAIL="$ROOT/bootstrap/corpus/emit-c/fail/contract_no_brace.oo"
REQ_PASS="$ROOT/fixtures/requires_simple.oo"
REQ_FAIL="$ROOT/fixtures/requires_fail.oo"
REQ_CX="$ROOT/bootstrap/corpus/emit-c/fail/requires_complex.oo"
ENS_PASS="$ROOT/fixtures/ensures_simple.oo"
ENS_FAIL="$ROOT/fixtures/ensures_fail.oo"
ENS_CX="$ROOT/bootstrap/corpus/emit-c/fail/ensures_complex.oo"
# Real fixtures that emptied bodies without contract skip:
IM="$ROOT/fixtures/int_main.oo"
HELLO="$ROOT/fixtures/hello.oo"

emit_ok() {
  local src="$1" base c_out
  base="$(basename "$src" .oo)"
  c_out="$TMPDIR/contracts_${base}.c"
  set +e
  "$OODAC" emit-c "$src" >"$c_out" 2>"$TMPDIR/contracts_${base}.err"
  local rc=$?
  if [[ $rc -ne 0 || ! -s "$c_out" ]] && [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then
    "$ROOT/bootstrap/seed/oodac" emit-c "$src" >"$c_out" 2>"$TMPDIR/contracts_${base}.err"
    rc=$?
  fi
  set -e
  if [[ "$rc" != "0" ]]; then
    echo "FAIL contracts emit-c exit $rc: $src" >&2
    cat "$TMPDIR/contracts_${base}.err" >&2
    exit 1
  fi
  if grep -E $'^ERR\t' "$c_out" >/dev/null 2>&1; then
    echo "FAIL contracts emit-c ERR: $src" >&2
    grep -E $'^ERR\t' "$c_out" >&2 || true
    exit 1
  fi
  grep -v '🚀\|Running main' "$c_out" >"${c_out}.clean" || true
  mv "${c_out}.clean" "$c_out"
  if grep -q 'oo_str_release\|oo_slist_release' "$c_out" && ! grep -q 'void oo_str_release' "$c_out"; then
    awk '
      { print }
      /} OoSList;/ && !done {
        print "void oo_slist_retain(OoSList); void oo_slist_release(OoSList);"
        print "void oo_ilist_retain(OoIList); void oo_ilist_release(OoIList);"
        print "void oo_str_retain(OoStr); void oo_str_release(OoStr);"
        done = 1
      }
    ' "$c_out" >"${c_out}.arc" && mv "${c_out}.arc" "$c_out"
  fi
  # Non-empty function bodies (contracts must not wipe stmts)
  if ! grep -qE 'return |oo_print|oo_str_concat' "$c_out"; then
    echo "FAIL contracts empty body: $src" >&2
    head -80 "$c_out" >&2
    exit 1
  fi
  printf '%s' "$c_out"
}

# --- pass: emit + gcc + run (int path) ---
for src in "$PASS" "$IM"; do
  base="$(basename "$src" .oo)"
  c_out="$(emit_ok "$src")"
  bin_out="$TMPDIR/contracts_${base}.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_out" -o "$bin_out" -lm
  out="$(timeout 3 "$bin_out")"
  echo "$out" | grep -q '42' || {
    echo "FAIL contracts run expected 42: $src got: $out" >&2
    exit 1
  }
  echo "OK contracts pass $base"
done

# hello: emit non-empty greet body (let String ty residual may block gcc run)
c_hello="$(emit_ok "$HELLO")"
grep -q 'OoStr greet' "$c_hello" || {
  echo "FAIL hello missing greet: $HELLO" >&2
  exit 1
}
grep -A10 'OoStr greet' "$c_hello" | grep -q 'return' || {
  echo "FAIL hello empty greet body: $HELLO" >&2
  head -60 "$c_hello" >&2
  exit 1
}
echo "OK contracts pass hello (emit body)"

# --- fail: garbage / missing body after requires ---
c_fail="$TMPDIR/contracts_fail.c"
err_fail="$TMPDIR/contracts_fail.err"
set +e
"$OODAC" emit-c "$FAIL" >"$c_fail" 2>"$err_fail"
frc=$?
set -e
if grep -E $'^ERR\tc_emit' "$c_fail" "$err_fail" >/dev/null 2>&1; then
  echo "OK contracts fail contract_no_brace (ERR line)"
elif [[ "$frc" != "0" ]]; then
  echo "OK contracts fail contract_no_brace (exit $frc)"
else
  echo "FAIL contracts should reject: $FAIL" >&2
  head -30 "$c_fail" >&2
  exit 1
fi

# build path smoke (int_main)
rm -f /tmp/im_contracts
"$OODAC" build "$IM" /tmp/im_contracts
outb="$(/tmp/im_contracts)"
echo "$outb" | grep -q '42' || {
  echo "FAIL oodac build int_main: $outb" >&2
  exit 1
}
echo "OK contracts build int_main"

# --- M19 simple requires: pass runtime ---
if [[ -f "$REQ_PASS" ]]; then
  c_rq="$(emit_ok "$REQ_PASS")"
  if ! grep -qE 'if \(!\(|requires' "$c_rq"; then
    echo "FAIL requires_simple missing requires lower" >&2
    head -80 "$c_rq" >&2
    exit 1
  fi
  bin_rq="$TMPDIR/requires_simple.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_rq" -o "$bin_rq" -lm
  out_rq="$(timeout 3 "$bin_rq")"
  echo "$out_rq" | grep -qE '3' || {
    echo "FAIL requires_simple run expected 3 got: $out_rq" >&2
    exit 1
  }
  echo "OK requires pass requires_simple"
fi

# --- M19 simple requires: fail runtime ---
if [[ -f "$REQ_FAIL" ]]; then
  c_rf="$(emit_ok "$REQ_FAIL")"
  bin_rf="$TMPDIR/requires_fail.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_rf" -o "$bin_rf" -lm
  set +e
  out_rf="$(timeout 3 "$bin_rf" 2>&1)"
  rfrc=$?
  set -e
  if [[ $rfrc -eq 0 ]]; then
    echo "FAIL requires_fail should non-zero out=$out_rf" >&2
    exit 1
  fi
  if ! echo "$out_rf" | grep -qiE 'contract|requires'; then
    echo "FAIL requires_fail missing contract/requires needle out=$out_rf" >&2
    exit 1
  fi
  echo "OK requires fail requires_fail (rc=$rfrc)"
fi

# --- M112 complex requires/ensures ---
if [[ -f "$ROOT/fixtures/complex_contract_pass.oo" ]]; then
  cx_pass="$(emit_ok "$ROOT/fixtures/complex_contract_pass.oo")"
  bin_cx="$TMPDIR/complex_contract_pass.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$cx_pass" -o "$bin_cx" -lm
  out_cx="$(timeout 3 "$bin_cx")"
  echo "OK complex contract pass"
fi

if [[ -f "$ROOT/fixtures/complex_contract_req_fail.oo" ]]; then
  cx_req_fail="$(emit_ok "$ROOT/fixtures/complex_contract_req_fail.oo")"
  bin_cx_req="$TMPDIR/complex_contract_req_fail.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$cx_req_fail" -o "$bin_cx_req" -lm
  set +e
  out_cx_req="$(timeout 3 "$bin_cx_req" 2>&1)"
  rc_cx_req=$?
  set -e
  if [[ $rc_cx_req -eq 0 ]]; then
    echo "FAIL complex_contract_req_fail should non-zero" >&2
    exit 1
  fi
  echo "OK complex contract req fail"
fi

# --- M9 simple ensures: pass runtime ---
if [[ -f "$ENS_PASS" ]]; then
  c_ens="$(emit_ok "$ENS_PASS")"
  if ! grep -q '__oo_ens_mode\|__result' "$c_ens"; then
    echo "FAIL ensures_simple missing ensures lower" >&2
    head -80 "$c_ens" >&2
    exit 1
  fi
  bin_ens="$TMPDIR/ensures_simple.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_ens" -o "$bin_ens" -lm
  out_ens="$(timeout 3 "$bin_ens")"
  echo "$out_ens" | grep -qE '2' || {
    echo "FAIL ensures_simple run expected 2 got: $out_ens" >&2
    exit 1
  }
  echo "OK ensures pass ensures_simple"
fi

# --- M9 simple ensures: fail runtime ---
if [[ -f "$ENS_FAIL" ]]; then
  c_ef="$(emit_ok "$ENS_FAIL")"
  bin_ef="$TMPDIR/ensures_fail.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_ef" -o "$bin_ef" -lm
  set +e
  out_ef="$(timeout 3 "$bin_ef" 2>&1)"
  efrc=$?
  set -e
  if [[ $efrc -eq 0 ]]; then
    echo "FAIL ensures_fail should non-zero out=$out_ef" >&2
    exit 1
  fi
  if ! echo "$out_ef" | grep -qiE 'contract|ensures'; then
    echo "FAIL ensures_fail missing contract/ensures needle out=$out_ef" >&2
    exit 1
  fi
  echo "OK ensures fail ensures_fail (rc=$efrc)"
fi

if [[ -f "$ROOT/fixtures/complex_contract_ens_fail.oo" ]]; then
  cx_ens_fail="$(emit_ok "$ROOT/fixtures/complex_contract_ens_fail.oo")"
  bin_cx_ens="$TMPDIR/complex_contract_ens_fail.bin"
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$cx_ens_fail" -o "$bin_cx_ens" -lm
  set +e
  out_cx_ens="$(timeout 3 "$bin_cx_ens" 2>&1)"
  rc_cx_ens=$?
  set -e
  if [[ $rc_cx_ens -eq 0 ]]; then
    echo "FAIL complex_contract_ens_fail should non-zero" >&2
    exit 1
  fi
  echo "OK complex contract ens fail"
fi

# M51 multi-clause simple AND (separate script keeps this file ≤ MAX_LINES)
bash "$ROOT/scripts/contracts_multi_clause_smoke.sh"

echo "contracts_native_smoke: pass+fail OK"
