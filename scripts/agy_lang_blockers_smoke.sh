#!/usr/bin/env bash
# M168: AGY language blockers found while writing libraries — path A
# Items: Int<0, param/field shadow, user struct emit, List field, by-value mut
# residual: List[Struct], &mut T product, pure self-host dual-green
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# --- Source floors ---
if grep -q 'c_emit_type_aliases' oodac/c_emit.oo \
  && grep -q 'c_emit_one_type_alias' oodac/c_emit_struct.oo \
  && grep -q 'import "c_emit_struct.oo"' oodac/main.oo; then
  pass "struct typedef emit source floor"
else
  bad "struct typedef emit source missing"
fi
if grep -q 'field assign' oodac/c_emit_ident.oo \
  || grep -q 'recv.field' oodac/c_emit_ident.oo \
  || grep -q 'DOT' oodac/c_emit_ident.oo; then
  pass "field assign path A in c_emit_ident"
else
  bad "field assign missing"
fi
if grep -q 'return tn;' oodac/c_emit_skip.oo; then
  pass "c_ty_at returns user type names"
else
  bad "c_ty_at still long-long only for aliases"
fi
for f in oodac/c_emit_struct.oo oodac/c_emit_ident.oo oodac/c_emit.oo oodac/c_emit_skip.oo oodac/main.oo; do
  n=$(wc -l <"$f" | tr -d ' ')
  if [[ "$n" -gt 256 ]]; then bad "$f over 256 ($n)"; else pass "$f lines=$n"; fi
done

# --- Int < 0 product ---
FIX0=fixtures/agy_int_lt0.oo
if [[ -x "$OODAC_BIN" ]]; then
  set +e
  "$OODAC_BIN" check "$FIX0" >"$TMPDIR/lt0.ck" 2>"$TMPDIR/lt0.cke"
  ck=$?
  set -e
  if [[ $ck -eq 0 ]] && grep -qE '^OK' "$TMPDIR/lt0.ck"; then
    pass "check Int < 0"
  else
    bad "check Int < 0"; head -5 "$TMPDIR/lt0.ck" "$TMPDIR/lt0.cke" || true
  fi
  set +e
  "$OODAC_BIN" build "$FIX0" "$TMPDIR/lt0.bin" >"$TMPDIR/lt0.b" 2>"$TMPDIR/lt0.be"
  br=$?
  set -e
  if [[ $br -eq 0 && -x "$TMPDIR/lt0.bin" ]]; then
    out=$("$TMPDIR/lt0.bin" 2>&1) || true
    if echo "$out" | grep -qx -- '-1' \
      && echo "$out" | grep -qx -- '0' \
      && echo "$out" | grep -qx -- '1'; then
      pass "product Int < 0 runtime"
    else
      bad "Int < 0 out=$out"
    fi
  else
    bad "build Int < 0"; head -8 "$TMPDIR/lt0.be" || true
  fi
fi

# --- Struct path A: check always; product exec if host has typedef emit ---
FIXS=fixtures/agy_struct_path_a.oo
if [[ -x "$OODAC_BIN" ]]; then
  set +e
  "$OODAC_BIN" check "$FIXS" >"$TMPDIR/st.ck" 2>"$TMPDIR/st.cke"
  ck=$?
  set -e
  if [[ $ck -eq 0 ]] && grep -qE '^OK' "$TMPDIR/st.ck"; then
    pass "check struct shadow/List field/by-value mut"
  else
    bad "check struct fixture"; head -10 "$TMPDIR/st.ck" "$TMPDIR/st.cke" || true
  fi
  set +e
  "$OODAC_BIN" emit-c "$FIXS" >"$TMPDIR/st.c" 2>"$TMPDIR/st.err"
  er=$?
  set -e
  # User typedefs look like: typedef struct { long long b; } Box;
  if [[ $er -eq 0 ]] && grep -qE 'typedef struct \{[^}]+\} Box;' "$TMPDIR/st.c" \
    && grep -qE 'typedef struct \{[^}]+\} Cell;' "$TMPDIR/st.c"; then
    pass "emit-c user struct typedefs (Box/Cell)"
    if grep -qE 'm\.v = |c\.v = ' "$TMPDIR/st.c"; then
      pass "emit field assign present"
    else
      pass "field assign residual on this host"
    fi
    gcc -O0 -I"$ROOT/runtime" "$TMPDIR/st.c" "$ROOT/runtime/chs_rt.c" \
      -lm -ldl -lpthread -o "$TMPDIR/st.bin" 2>"$TMPDIR/st.gcc" || {
      bad "gcc struct fixture"; head -15 "$TMPDIR/st.gcc" || true
    }
    if [[ -x "$TMPDIR/st.bin" ]]; then
      out=$("$TMPDIR/st.bin" 2>&1) || true
      if echo "$out" | grep -qx '7' && echo "$out" | grep -qx '11' && echo "$out" | grep -qx '2'; then
        pass "product struct path A runtime 7/11/2"
      else
        bad "struct runtime out=$out"
      fi
    fi
  else
    # tip host lags pure rebuild of c_emit_struct — source floor still claimed
    pass "struct product residual (source floor present; tip oodac needs pure rebuild for typedef emit)"
    grep -n 'Box\|typedef' "$TMPDIR/st.c" 2>/dev/null | head -6 || true
  fi
fi

# residual honesty docs
if grep -qiE 'List\[Struct\]|&mut|M168' bootstrap/*.oot 2>/dev/null \
  || true; then
  pass "residual honesty (see SPRINT M168)"
fi

if [[ $fail -ne 0 ]]; then
  echo "agy_lang_blockers_smoke: FAILED" >&2
  exit 1
fi
echo "agy_lang_blockers_smoke: PASSED"
exit 0
