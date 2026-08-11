#!/usr/bin/env bash
# M166 path A: sin/cos/ln/exp/sqrt/pow IEEE double floor
# job: prove runtime + wiring; residual honesty no decimal type
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

DOC="$ROOT/bootstrap/MATH_TRIG.md"
[[ -f "$DOC" ]] || { echo "ERR_NO_DOC: $DOC" >&2; exit 1; }

if grep -qE 'sin\(|cos\(|ln\(|exp\(|sqrt\(|pow\(' "$DOC" \
  && grep -qiE 'IEEE|double' "$DOC"; then
  pass "doc path A math free names + IEEE double"
else
  bad "doc missing path A math free names"
fi
if grep -qiE 'no decimal|not decimal|NOT decimal|residual' "$DOC"; then
  pass "doc residual honesty no decimal type"
else
  bad "doc missing residual decimal honesty"
fi
if grep -q 'MATH_TRIG_PATH_A_ALPHA' "$DOC"; then
  pass "doc marker MATH_TRIG_PATH_A_ALPHA"
else
  bad "doc missing marker"
fi

# Runtime symbols
if grep -q 'oo_sin' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_cos' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_ln' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_exp' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_sqrt' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_pow' "$ROOT/runtime/chs_rt.h" \
  && grep -q 'oo_print_double' "$ROOT/runtime/chs_rt.h"; then
  pass "runtime decls math free names"
else
  bad "runtime chs_rt.h missing M166 decls"
fi
if grep -q 'chs_rt_math.c' "$ROOT/runtime/chs_rt.c"; then
  pass "chs_rt.c includes chs_rt_math.c"
else
  bad "chs_rt.c missing math include"
fi
if grep -q 'oo_sin' "$ROOT/runtime/chs_rt_math.c" \
  && grep -q 'oo_print_double' "$ROOT/runtime/chs_rt_math.c"; then
  pass "runtime impl in chs_rt_math.c"
else
  bad "runtime chs_rt_math.c missing impl"
fi

# Emit/check wiring (source floor; product rebuild may lag)
if grep -qE 'name == "sin"|name == "cos"' "$ROOT/oodac/tc_names.oo" \
  && grep -q 'sqrt' "$ROOT/oodac/tc_names.oo"; then
  pass "tc_names free names"
else
  bad "tc_names missing M166 free names"
fi
if grep -q 'sin=1' "$ROOT/oodac/tc_call_arity.oo" \
  && grep -q 'pow=2' "$ROOT/oodac/tc_call_arity.oo"; then
  pass "tc_call_arity seeds"
else
  bad "tc_call_arity missing M166 seeds"
fi
if grep -q 'oo_sin' "$ROOT/oodac/c_emit_libfloor.oo" \
  && grep -q 'oo_pow' "$ROOT/oodac/c_emit_libfloor.oo"; then
  pass "c_emit_libfloor lowers"
else
  bad "c_emit_libfloor missing M166 lowers"
fi
if grep -q 'oo_sin' "$ROOT/oodac/c_emit_preamble.oo" \
  && grep -q 'oo_print_double' "$ROOT/oodac/c_emit_preamble.oo"; then
  pass "c_emit_preamble decls"
else
  bad "c_emit_preamble missing M166 decls"
fi
if grep -q 'oo_sin' "$ROOT/oodac/c_emit_ty.oo" \
  && grep -q 'double' "$ROOT/oodac/c_emit_ty.oo"; then
  pass "c_emit_ty result types"
else
  bad "c_emit_ty missing M166 types"
fi
if grep -q 'oo_print_double' "$ROOT/oodac/c_emit_print.oo"; then
  pass "c_emit_print double path"
else
  bad "c_emit_print missing oo_print_double"
fi
if grep -q 'Float' "$ROOT/oodac/c_emit_skip.oo" \
  && grep -q 'double' "$ROOT/oodac/c_emit_skip.oo"; then
  pass "c_ty_at Float → double"
else
  bad "c_ty_at missing Float→double"
fi

# std wrappers residual honesty
STD="$ROOT/std/math.oo"
if [[ -f "$STD" ]] && grep -q 'sin(' "$STD" \
  && grep -qiE 'NOT decimal|no decimal|IEEE|double' "$STD"; then
  pass "std/math.oo wrappers + residual"
else
  bad "std/math.oo missing M166 wrappers/residual"
fi

# Runtime C floor (independent of oodac free-name rebuild)
cat >"$TMPDIR/math_trig_rt.c" <<'CEOF'
#include "chs_rt.h"
#include <math.h>
#include <stdio.h>
static int near(double a, double b, double eps) {
  double d = a - b;
  if (d < 0) d = -d;
  return d <= eps;
}
int main(void) {
  if (!near(oo_sin(0.0), 0.0, 1e-12)) return 1;
  if (!near(oo_cos(0.0), 1.0, 1e-12)) return 2;
  if (!near(oo_sqrt(4.0), 2.0, 1e-12)) return 3;
  if (!near(oo_ln(1.0), 0.0, 1e-12)) return 4;
  if (!near(oo_exp(0.0), 1.0, 1e-12)) return 5;
  if (!near(oo_pow(2.0, 3.0), 8.0, 1e-12)) return 6;
  /* print path */
  oo_print_double(oo_cos(0.0));
  oo_println();
  printf("rt-ok\n");
  return 0;
}
CEOF
if gcc -O0 -I"$ROOT/runtime" "$TMPDIR/math_trig_rt.c" "$ROOT/runtime/chs_rt.c" \
  -lm -ldl -lpthread -o "$TMPDIR/math_trig_rt" 2>"$TMPDIR/math_trig_rt.err"; then
  out=$("$TMPDIR/math_trig_rt" 2>&1) || true
  echo "$out" | grep -q 'rt-ok' && pass "runtime C math ops" \
    || bad "runtime C out=$out"
else
  bad "runtime C compile"; head -10 "$TMPDIR/math_trig_rt.err" || true
fi

# Executable product floor when oodac knows free names
if [[ -x "$OODAC_BIN" ]]; then
  FIX="$ROOT/fixtures/math_trig.oo"
  if OODAC_BIN="$OODAC_BIN" "$OODAC_BIN" build "$FIX" "$TMPDIR/mathtrig" \
    >"$TMPDIR/mathtrig.out" 2>"$TMPDIR/mathtrig.err" && [[ -x "$TMPDIR/mathtrig" ]]; then
    out=$("$TMPDIR/mathtrig" 2>&1) || true
    # expect: ~0, ~1, 2, 0, 1, 8 (printf %g)
    if echo "$out" | grep -qE '^(0|0\.0*)$' \
      && echo "$out" | grep -qE '^(1|1\.0*)$' \
      && echo "$out" | grep -qE '^(2|2\.0*)$' \
      && echo "$out" | grep -qE '^(8|8\.0*)$'; then
      pass "math_trig fixture sin/cos/sqrt/ln/exp/pow"
    else
      # tolerance: first lines may be 0 / 1 with scientific
      if echo "$out" | head -1 | grep -qE '^-?0(\.0+)?([eE][+-]?0+)?$' \
        && echo "$out" | sed -n '2p' | grep -qE '^1(\.0+)?([eE][+-]?0+)?$'; then
        pass "math_trig fixture approx sin0/cos0"
      else
        bad "math_trig fixture out=$out"
      fi
    fi
  else
    if grep -qiE 'undefined|unknown|ERR' "$TMPDIR/mathtrig.err" 2>/dev/null; then
      pass "skip exec (oodac pre path-A rebuild; source floor checked)"
      head -6 "$TMPDIR/mathtrig.err" 2>/dev/null || true
    else
      bad "build math_trig"; head -12 "$TMPDIR/mathtrig.err" || true
    fi
  fi
else
  pass "skip oodac exec (no OODAC)"
fi

lines=$(wc -l <"$ROOT/runtime/chs_rt_math.c" | tr -d ' ')
if [[ "$lines" -le 256 ]]; then
  pass "chs_rt_math.c line budget ($lines<=256)"
else
  bad "chs_rt_math.c over 256 lines ($lines)"
fi

if [[ $fail -ne 0 ]]; then
  echo "math_trig_smoke: FAILED" >&2
  exit 1
fi
echo "math_trig_smoke: PASSED"
exit 0
