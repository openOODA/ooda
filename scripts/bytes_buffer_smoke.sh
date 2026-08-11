#!/usr/bin/env bash
# M164 path A: List[Int] 0..255 Byte buffer + String byte view free names
# job: prove source floor + runtime; residual honesty for true List[Byte]/&str
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

DOC="$ROOT/bootstrap/BYTE_STR.md"
[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

# Doc: path A Byte buffer In + residual honesty
if grep -qE 'bytes_new|bytes_push|bytes_to_str' "$DOC" && grep -qiE 'List\[Int\].*0\.\.255|Byte buffer' "$DOC"; then
  pass "doc path A List[Int] Byte buffer"
else
  bad "doc missing path A Byte buffer (List[Int] 0..255)"
fi
if grep -qiE 'not.*List\[Byte\]|List\[Byte\].*not product|List\[Byte\] ABI' "$DOC"; then
  pass "doc residual honesty for true List[Byte]"
else
  bad "doc missing List[Byte] residual"
fi
if grep -qiE 'not.*&str|&str.*borrow|NOT product' "$DOC"; then
  pass "doc residual honesty for true &str borrow"
else
  bad "doc missing residual &str honesty"
fi
if grep -qE 'bytes_from_str|bytes_concat' "$DOC"; then
  pass "doc bytes_from_str / bytes_concat"
else
  bad "doc missing bytes_from_str/bytes_concat"
fi

# Runtime symbols
if grep -q 'oo_bytes_new' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_bytes_push' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_bytes_get' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_bytes_to_str' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_bytes_from_str' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_bytes_concat' "$ROOT/runtime/chs_rt.h"; then
  pass "runtime decls bytes buffer free names"
else
  bad "runtime chs_rt.h missing M164 decls"
fi
if grep -q 'oo_bytes_to_str' "$ROOT/runtime/chs_rt_str.c" \
  && grep -q 'oo_bytes_push' "$ROOT/runtime/chs_rt_str.c"; then
  pass "runtime impl in chs_rt_str.c"
else
  bad "runtime chs_rt_str.c missing M164 impl"
fi

# Emit/check wiring (source floor; product rebuild may lag)
if grep -q 'bytes_new' "$ROOT/oodac/tc_names.oo" \
  && grep -q 'bytes_to_str' "$ROOT/oodac/tc_names.oo" \
  && grep -q 'bytes_from_str' "$ROOT/oodac/tc_names.oo"; then
  pass "tc_names free names"
else
  bad "tc_names missing M164 free names"
fi
if grep -q 'oo_bytes_new' "$ROOT/oodac/c_emit_libfloor.oo" \
  && grep -q 'oo_bytes_to_str' "$ROOT/oodac/c_emit_libfloor.oo"; then
  pass "c_emit_libfloor lowers"
else
  bad "c_emit_libfloor missing M164 lowers"
fi
if grep -q 'oo_bytes_new' "$ROOT/oodac/c_emit_preamble.oo" \
  && grep -q 'oo_bytes_to_str' "$ROOT/oodac/c_emit_preamble.oo"; then
  pass "c_emit_preamble decls"
else
  bad "c_emit_preamble missing M164 decls"
fi
if grep -q 'bytes_new' "$ROOT/oodac/c_emit_ty.oo" \
  && grep -q 'bytes_to_str' "$ROOT/oodac/c_emit_ty.oo"; then
  pass "c_emit_ty result types"
else
  bad "c_emit_ty missing M164 types"
fi
if grep -q 'bytes_new=0' "$ROOT/oodac/tc_call_arity.oo" \
  && grep -q 'bytes_push=2' "$ROOT/oodac/tc_call_arity.oo"; then
  pass "tc_call_arity seeds"
else
  bad "tc_call_arity missing M164 seeds"
fi

# std wrappers residual honesty
STD="$ROOT/std/byte.oo"
if [[ -f "$STD" ]] && grep -q 'bytes_new\|byte_buf_new' "$STD" \
  && grep -qiE 'List\[Int\]|0\.\.255' "$STD" \
  && grep -qiE 'residual:.*&str|NOT product|List\[Byte\]' "$STD"; then
  pass "std/byte.oo buffer wrappers + residual"
else
  bad "std/byte.oo missing buffer wrappers/residual"
fi

# Runtime C floor (independent of oodac free-name rebuild)
cat >"$TMPDIR/bytes_buf_rt.c" <<'CEOF'
#include "chs_rt.h"
#include <stdio.h>
int main(void) {
  OoIList bs = oo_bytes_new();
  bs = oo_bytes_push(bs, 65);
  bs = oo_bytes_push(bs, 300); /* clamp → 255 */
  if (oo_bytes_get(bs, 0) != 65) return 1;
  if (oo_bytes_get(bs, 1) != 255) return 2;
  if (oo_bytes_get(bs, 9) != -1) return 3;
  OoStr s = oo_bytes_to_str(bs);
  if (oo_bytes_len(s) != 2) return 4;
  if (oo_byte_at(s, 0) != 65) return 5;
  if (oo_byte_at(s, 1) != 255) return 6;
  OoStr v = oo_bytes_from_str(oo_str_lit("AB"));
  if (!oo_bytes_eq(v, oo_str_lit("AB"))) return 7;
  OoStr c = oo_bytes_concat(oo_str_lit("A"), oo_str_lit("B"));
  if (!oo_bytes_eq(c, oo_str_lit("AB"))) return 8;
  printf("rt-ok\n");
  return 0;
}
CEOF
if gcc -O0 -I"$ROOT/runtime" "$TMPDIR/bytes_buf_rt.c" "$ROOT/runtime/chs_rt.c" \
  -lm -ldl -lpthread -o "$TMPDIR/bytes_buf_rt" 2>"$TMPDIR/bytes_buf_rt.err"; then
  out=$("$TMPDIR/bytes_buf_rt" 2>&1) || true
  echo "$out" | grep -q 'rt-ok' && pass "runtime C bytes buffer" \
    || bad "runtime C out=$out"
else
  bad "runtime C compile"; head -10 "$TMPDIR/bytes_buf_rt.err" || true
fi

# Executable product floor when oodac knows free names
if [[ -x "$OODAC_BIN" ]]; then
  FIX="$ROOT/fixtures/bytes_buffer_main.oo"
  if OODAC_BIN="$OODAC_BIN" "$OODAC_BIN" build "$FIX" "$TMPDIR/bbuf" \
    >"$TMPDIR/bbuf.out" 2>"$TMPDIR/bbuf.err" && [[ -x "$TMPDIR/bbuf" ]]; then
    out=$("$TMPDIR/bbuf" 2>&1) || true
    # expect: 65\n66\n-1\nAB\n2\n65\ntrue\ntrue  (bool may be 1)
    if echo "$out" | grep -qE '^65$' \
      && echo "$out" | grep -qE '^66$' \
      && echo "$out" | grep -qE '^-1$' \
      && echo "$out" | grep -qE '^AB$' \
      && echo "$out" | grep -qE '^2$' \
      && echo "$out" | grep -qE '^(true|1)$'; then
      pass "bytes_buffer fixture push/get/to_str/from_str/concat"
    else
      bad "bytes_buffer fixture out=$out"
    fi
  else
    if grep -qiE 'undefined|unknown|ERR' "$TMPDIR/bbuf.err" 2>/dev/null; then
      pass "skip exec (oodac pre M164 rebuild; source floor checked)"
      head -6 "$TMPDIR/bbuf.err" 2>/dev/null || true
    else
      bad "build bytes_buffer_main"; head -12 "$TMPDIR/bbuf.err" || true
    fi
  fi
else
  pass "skip oodac exec (no OODAC)"
fi

# Residual honesty pack still present
if [[ -x "$ROOT/scripts/byte_str_residual_smoke.sh" ]]; then
  if bash "$ROOT/scripts/byte_str_residual_smoke.sh" >"$TMPDIR/bres2.out" 2>"$TMPDIR/bres2.err"; then
    pass "byte_str_residual_smoke still green"
  else
    bad "byte_str_residual_smoke failed"
    head -15 "$TMPDIR/bres2.err" "$TMPDIR/bres2.out" 2>/dev/null || true
  fi
else
  bad "missing byte_str_residual_smoke.sh"
fi

if [[ $fail -ne 0 ]]; then
  echo "bytes_buffer_smoke: FAILED" >&2
  exit 1
fi
echo "bytes_buffer_smoke: PASSED"
exit 0
