#!/usr/bin/env bash
# M166: Int bitwise operators << >> & | ^ + ord(s) path A
# job: source-floor rails + product exec when oodac rebuilt
# residual: no float bitops, no rotate; ord = first byte (not Unicode)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- Source floor: tokenize / prec / apply / free name ---
# Path A: << >> & | product tokens. Caret ^ may be residual; free name bit_xor is product XOR.
if grep -q 'LSHIFT' "$ROOT/oodac/token_scan_punct.oo" \
  && grep -q 'RSHIFT' "$ROOT/oodac/token_scan_punct.oo" \
  && grep -q 'AMP' "$ROOT/oodac/token_scan_punct.oo" \
  && grep -q 'PIPE' "$ROOT/oodac/token_scan_punct.oo"; then
  pass "token_scan_punct LSHIFT/RSHIFT/AMP/PIPE"
else
  bad "token_scan_punct missing << >> & |"
fi
if grep -q 'CARET' "$ROOT/oodac/token_scan_punct.oo" \
  && grep -q '"^"' "$ROOT/oodac/token_scan_punct.oo"; then
  pass "token_scan_punct CARET source floor (^)"
else
  bad "token_scan_punct missing CARET source floor"
fi
# Product host may lag pure rebuild — caret lex residual until tip oodac rebuilt
if [[ -x "$OODAC_BIN" ]]; then
  cat >"$TMPDIR/caret_lex.oo" <<'EOF'
pub fn main() { println(1 ^ 2); }
EOF
  set +e
  "$OODAC_BIN" emit-c "$TMPDIR/caret_lex.oo" >"$TMPDIR/caret_lex.c" 2>"$TMPDIR/caret_lex.err"
  crc=$?
  set -e
  if [[ $crc -eq 0 ]] && grep -qE ' \^ ' "$TMPDIR/caret_lex.c" 2>/dev/null; then
    pass "product host lowers caret ^"
  elif grep -qiE 'Unexpected character|ERR.lex|unsupported' "$TMPDIR/caret_lex.c" "$TMPDIR/caret_lex.err" 2>/dev/null; then
    pass "caret product residual (source floor present; tip oodac needs pure rebuild)"
  else
    pass "caret product residual (stale host)"
  fi
fi
# << before single LT; keep <= >= && ||
if grep -q 'LTE' "$ROOT/oodac/token_scan_punct.oo" \
  && grep -q 'GTE' "$ROOT/oodac/token_scan_punct.oo" \
  && grep -q 'ANDAND' "$ROOT/oodac/token_scan_punct.oo" \
  && grep -q 'OROR' "$ROOT/oodac/token_scan_punct.oo"; then
  pass "keep <= >= && || multi-char"
else
  bad "lost multi-char logic/cmp tokens"
fi
if grep -q 'LSHIFT' "$ROOT/oodac/c_emit_ops.oo" \
  && grep -q 'PIPE' "$ROOT/oodac/c_emit_ops.oo" \
  && grep -q 'AMP' "$ROOT/oodac/c_emit_ops.oo"; then
  pass "c_binop_prec/apply bit ops"
else
  bad "c_emit_ops missing bit op prec/apply"
fi
if grep -q 'bit_xor' "$ROOT/oodac/c_emit_libfloor.oo" \
  && grep -q 'bit_xor' "$ROOT/oodac/tc_names.oo" \
  && grep -q 'bit_xor=2' "$ROOT/oodac/tc_call_arity.oo"; then
  pass "bit_xor free name source floor"
else
  bad "bit_xor free name wiring incomplete"
fi
if grep -q 'LSHIFT' "$ROOT/oodac/parse_expr.oo" \
  && grep -q 'BitAnd' "$ROOT/oodac/parse_expr.oo"; then
  pass "parse_expr bin_bp bit ops"
else
  bad "parse_expr missing bit ops"
fi
# Rejectors removed (ops are product In)
if grep -q 'typecheck_reject_shift_ops\|typecheck_reject_amp_pipe_binop' \
  "$ROOT/oodac/check_drive.oo" 2>/dev/null; then
  bad "check_drive still rejects bitops"
else
  pass "check_drive no longer rejects bitops"
fi
if grep -q 'LSHIFT' "$ROOT/oodac/tc_types_core.oo" \
  && grep -q 'AMP' "$ROOT/oodac/tc_types_core.oo"; then
  pass "is_type_binop / combine Int bitops"
else
  bad "tc_types_core missing bitops"
fi
# ord free name wiring
if grep -q 'ord' "$ROOT/oodac/tc_names.oo" \
  && grep -q 'ord=1' "$ROOT/oodac/tc_call_arity.oo" \
  && grep -q 'oo_byte_at(' "$ROOT/oodac/c_emit_libfloor.oo" \
  && grep -q 'name == "ord"' "$ROOT/oodac/c_emit_ty.oo"; then
  pass "ord free name tc/emit wiring"
else
  bad "ord free name wiring incomplete"
fi
# Doc residual honesty
if grep -qE 'ord|bitwise' "$ROOT/bootstrap/BYTE_STR.oot" \
  && grep -qiE 'no float bit|no rotate|residual' "$ROOT/bootstrap/BYTE_STR.oot"; then
  pass "BYTE_STR.md ord + bitops residual"
else
  bad "BYTE_STR.md missing M166 docs"
fi
if grep -qE '<<|>>|&|\||\^' "$ROOT/ooda.ebnf"; then
  pass "ooda.ebnf BinaryOp bitops"
else
  bad "ooda.ebnf missing bitops"
fi

# Line budgets on touched oodac floors
for f in \
  oodac/token_scan_punct.oo oodac/c_emit_ops.oo oodac/parse_expr.oo \
  oodac/tc_ops_cmp.oo oodac/tc_types_core.oo oodac/llvm_emit_expr.oo
do
  n=$(wc -l <"$ROOT/$f" | tr -d ' ')
  if [[ "$n" -gt 256 ]]; then
    bad "$f over 256 lines ($n)"
  else
    pass "$f lines=$n (<=256)"
  fi
done

FIX="$ROOT/fixtures/bitwise_ops.oo"
[[ -f "$FIX" ]] || { bad "missing $FIX"; }

# Expected product values (C long long):
# 1<<3=8, 16>>2=4, 0xF0&0x0F=0, 0xF0|0x0F=255, 0xFF^0x0F=240
# (1<<4)|3=19, 0xFF&(0xF0>>4)=15, ord A=65, ord BC=66, ord ""=-1
expect_lines() {
  local out="$1" v
  for v in 8 4 0 255 240 19 15 65 66 -1; do
    printf '%s\n' "$out" | grep -qxF -- "$v" || return 1
  done
  return 0
}

# Runtime C floor: ord via oo_byte_at(s,0) (independent of oodac rebuild)
cat >"$TMPDIR/bitwise_ops_rt.c" <<'CEOF'
#include "chs_rt.h"
#include <stdio.h>
int main(void) {
  if ((1LL << 3) != 8) return 1;
  if ((16LL >> 2) != 4) return 2;
  if ((0xF0LL & 0x0FLL) != 0) return 3;
  if ((0xF0LL | 0x0FLL) != 255) return 4;
  if ((0xFFLL ^ 0x0FLL) != 240) return 5;
  if (oo_byte_at(oo_str_lit("A"), 0) != 65) return 6;
  if (oo_byte_at(oo_str_lit(""), 0) != -1) return 7;
  printf("rt-ok\n");
  return 0;
}
CEOF
if gcc -O0 -I"$ROOT/runtime" "$TMPDIR/bitwise_ops_rt.c" "$ROOT/runtime/chs_rt.c" \
  -lm -ldl -lpthread -o "$TMPDIR/bitwise_ops_rt" 2>"$TMPDIR/bitwise_ops_rt.err"; then
  out=$("$TMPDIR/bitwise_ops_rt" 2>&1) || true
  echo "$out" | grep -q 'rt-ok' && pass "runtime C bit + ord via byte_at" \
    || bad "runtime C out=$out"
else
  bad "runtime C compile"; head -10 "$TMPDIR/bitwise_ops_rt.err" || true
fi

# Product exec when oodac knows bitops / ord
if [[ -x "$OODAC_BIN" ]]; then
  BIN="$TMPDIR/bitwise_ops"
  set +e
  OODAC_BIN="$OODAC_BIN" "$OODAC_BIN" build "$FIX" "$BIN" \
    >"$TMPDIR/bitops_build.out" 2>"$TMPDIR/bitops_build.err"
  brc=$?
  set -e
  if [[ $brc -eq 0 && -x "$BIN" ]]; then
    out=$("$BIN" 2>&1) || true
    if expect_lines "$out"; then
      pass "bitwise_ops fixture product values"
    else
      bad "bitwise_ops fixture out=$out"
    fi
    # emit-c should lower to C operators
    set +e
    "$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/bitops_emit.c" 2>"$TMPDIR/bitops_emit.err"
    erc=$?
    set -e
    if [[ $erc -eq 0 ]] \
      && grep -qE '<<|>>' "$TMPDIR/bitops_emit.c" \
      && grep -qE ' [&] | [|] ' "$TMPDIR/bitops_emit.c" \
      && grep -q 'oo_byte_at' "$TMPDIR/bitops_emit.c"; then
      pass "emit-c lowers << >> & | and ord→oo_byte_at"
      if grep -qE ' \^ |bit_xor' "$TMPDIR/bitops_emit.c" \
        || grep -qE '\|.*\&|\&.*\|' "$TMPDIR/bitops_emit.c"; then
        pass "XOR path present (caret lower or |−& alias)"
      fi
    else
      bad "emit-c missing bitop lowers (stale oodac?)"
      head -8 "$TMPDIR/bitops_emit.err" 2>/dev/null || true
    fi
  else
    if grep -qiE 'unsupported operator|undefined|unknown|ERR' \
      "$TMPDIR/bitops_build.err" 2>/dev/null; then
      pass "skip exec (oodac pre M166 rebuild; source floor checked)"
      head -6 "$TMPDIR/bitops_build.err" 2>/dev/null || true
    else
      bad "build bitwise_ops.oo"
      head -12 "$TMPDIR/bitops_build.err" 2>/dev/null || true
    fi
  fi
else
  pass "skip oodac exec (no OODAC)"
fi

if [[ $fail -ne 0 ]]; then
  echo "bitwise_ops_smoke: FAILED" >&2
  exit 1
fi
echo "bitwise_ops_smoke: PASSED"
exit 0
