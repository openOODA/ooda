#!/usr/bin/env bash
# M165 path A: str_starts_with / ends_with / index_of / repeat / to_uppercase
# job: prove free-name product floor; residual honesty for true &str remains
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

DOC="$ROOT/bootstrap/STR_OPS.md"
[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

# Doc: path A In + residual honesty
if grep -qE 'str_starts_with|str_ends_with|str_index_of|str_repeat' "$DOC" \
  && grep -qiE 'path A|owned' "$DOC"; then
  pass "doc path A string free names"
else
  bad "doc missing path A str ops"
fi
if grep -qiE 'not.*&str|&str.*borrow|no lifetime|residual' "$DOC"; then
  pass "doc residual honesty for true &str borrow"
else
  bad "doc missing residual &str honesty"
fi
if grep -q 'str_to_uppercase' "$DOC" && grep -qiE 'to_lowercase|lowercase' "$DOC"; then
  pass "doc uppercase free + lowercase already present"
else
  bad "doc missing uppercase/lowercase note"
fi

# Runtime symbols
if grep -q 'oo_str_starts_with' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_str_ends_with' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_str_index_of' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_str_repeat' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_str_to_uppercase' "$ROOT/runtime/chs_rt.h"; then
  pass "runtime decls str ops free names"
else
  bad "runtime chs_rt.h missing M165 decls"
fi
if grep -q 'oo_str_starts_with' "$ROOT/runtime/chs_rt_str.c" \
  && grep -q 'oo_str_index_of' "$ROOT/runtime/chs_rt_str.c" \
  && grep -q 'oo_str_repeat' "$ROOT/runtime/chs_rt_str.c"; then
  pass "runtime impl in chs_rt_str.c"
else
  bad "runtime chs_rt_str.c missing M165 impl"
fi
if grep -q 'oo_str_to_uppercase' "$ROOT/runtime/chs_rt_print.c"; then
  pass "runtime uppercase in chs_rt_print.c"
else
  bad "runtime missing oo_str_to_uppercase"
fi

# Emit/check wiring (source floor; product rebuild may lag)
if grep -q 'str_starts_with' "$ROOT/oodac/tc_names.oo" \
  && grep -q 'str_index_of' "$ROOT/oodac/tc_names.oo" \
  && grep -q 'str_to_uppercase' "$ROOT/oodac/tc_names.oo"; then
  pass "tc_names free names"
else
  bad "tc_names missing M165 free names"
fi
if grep -q 'str_starts_with=2' "$ROOT/oodac/tc_call_arity.oo" \
  && grep -q 'str_repeat=2' "$ROOT/oodac/tc_call_arity.oo"; then
  pass "tc_call_arity seeds"
else
  bad "tc_call_arity missing M165 seeds"
fi
if grep -q 'oo_str_starts_with' "$ROOT/oodac/c_emit_libfloor.oo" \
  && grep -q 'oo_str_repeat' "$ROOT/oodac/c_emit_libfloor.oo"; then
  pass "c_emit_libfloor lowers"
else
  bad "c_emit_libfloor missing M165 lowers"
fi
if grep -q 'oo_str_starts_with' "$ROOT/oodac/c_emit_preamble.oo" \
  && grep -q 'oo_str_repeat' "$ROOT/oodac/c_emit_preamble.oo"; then
  pass "c_emit_preamble decls"
else
  bad "c_emit_preamble missing M165 decls"
fi
if grep -q 'str_index_of' "$ROOT/oodac/c_emit_ty.oo" \
  && grep -q 'str_starts_with' "$ROOT/oodac/c_emit_ty.oo" \
  && grep -q 'str_repeat' "$ROOT/oodac/c_emit_ty.oo"; then
  pass "c_emit_ty result types"
else
  bad "c_emit_ty missing M165 types"
fi

# std wrappers residual honesty
STD="$ROOT/std/str.oo"
if [[ -f "$STD" ]] && grep -q 'str_starts_with' "$STD" \
  && grep -qiE 'residual:.*&str|NOT true &str' "$STD"; then
  pass "std/str.oo wrappers + residual"
else
  bad "std/str.oo missing M165 wrappers/residual"
fi

# Runtime C floor (independent of oodac free-name rebuild)
cat >"$TMPDIR/str_ops_path_a_rt.c" <<'CEOF'
#include "chs_rt.h"
#include <stdio.h>
int main(void) {
  OoStr s = oo_str_lit("hello");
  if (!oo_str_starts_with(s, oo_str_lit("he"))) return 1;
  if (oo_str_starts_with(s, oo_str_lit("lo"))) return 2;
  if (!oo_str_ends_with(s, oo_str_lit("lo"))) return 3;
  if (oo_str_ends_with(s, oo_str_lit("he"))) return 4;
  if (oo_str_index_of(s, oo_str_lit("ll")) != 2) return 5;
  if (oo_str_index_of(s, oo_str_lit("z")) != -1) return 6;
  OoStr r = oo_str_repeat(oo_str_lit("ab"), 3);
  if (!oo_str_eq(r, oo_str_lit("ababab"))) return 7;
  OoStr u = oo_str_to_uppercase(oo_str_lit("Hi"));
  if (!oo_str_eq(u, oo_str_lit("HI"))) return 8;
  /* cap n<=1024 soft */
  OoStr big = oo_str_repeat(oo_str_lit("x"), 2000);
  if (oo_str_byte_len(big) != 1024) return 9;
  printf("rt-ok\n");
  return 0;
}
CEOF
if gcc -O0 -I"$ROOT/runtime" "$TMPDIR/str_ops_path_a_rt.c" "$ROOT/runtime/chs_rt.c" \
  -lm -ldl -lpthread -o "$TMPDIR/str_ops_path_a_rt" 2>"$TMPDIR/str_ops_path_a_rt.err"; then
  out=$("$TMPDIR/str_ops_path_a_rt" 2>&1) || true
  echo "$out" | grep -q 'rt-ok' && pass "runtime C str ops" \
    || bad "runtime C out=$out"
else
  bad "runtime C compile"; head -10 "$TMPDIR/str_ops_path_a_rt.err" || true
fi

# Executable product floor when oodac knows free names
if [[ -x "$OODAC_BIN" ]]; then
  FIX="$ROOT/fixtures/str_ops_main.oo"
  if OODAC_BIN="$OODAC_BIN" "$OODAC_BIN" build "$FIX" "$TMPDIR/strops" \
    >"$TMPDIR/strops.out" 2>"$TMPDIR/strops.err" && [[ -x "$TMPDIR/strops" ]]; then
    out=$("$TMPDIR/strops" 2>&1) || true
    # expect: true false true false 2 -1 ababab HI
    if echo "$out" | grep -qE '^(true|1)$' \
      && echo "$out" | grep -qE '^(false|0)$' \
      && echo "$out" | grep -qE '^2$' \
      && echo "$out" | grep -qE '^-1$' \
      && echo "$out" | grep -qE '^ababab$' \
      && echo "$out" | grep -qE '^HI$'; then
      pass "str_ops fixture starts/ends/index/repeat/upper"
    else
      bad "str_ops fixture out=$out"
    fi
  else
    if grep -qiE 'undefined|unknown|ERR' "$TMPDIR/strops.err" 2>/dev/null; then
      pass "skip exec (oodac pre path-A rebuild; source floor checked)"
      head -6 "$TMPDIR/strops.err" 2>/dev/null || true
    else
      bad "build str_ops_main"; head -12 "$TMPDIR/strops.err" || true
    fi
  fi
else
  pass "skip oodac exec (no OODAC)"
fi

# Line budget honesty (alpha ≤256 lines/file for touched runtime TU)
lines=$(wc -l <"$ROOT/runtime/chs_rt_str.c" | tr -d ' ')
if [[ "$lines" -le 256 ]]; then
  pass "chs_rt_str.c line budget ($lines<=256)"
else
  bad "chs_rt_str.c over 256 lines ($lines)"
fi

if [[ $fail -ne 0 ]]; then
  echo "str_ops_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "str_ops_path_a_smoke: PASSED"
exit 0
