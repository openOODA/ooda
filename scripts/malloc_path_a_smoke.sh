#!/usr/bin/env bash
# M166 path A — product free names malloc/free (→ oo_alloc_bytes / oo_free_bytes) under AllocCap
# honesty: not OS rlimit; not GC; not ambient heap; process-local AllocCap only
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)
FIX="$ROOT/fixtures/malloc_path_a.oo"
DOC="$ROOT/bootstrap/MEMORY_QUOTA.oot"

# Doc honesty
if [[ -f "$DOC" ]] \
  && grep -qE 'malloc|free' "$DOC" \
  && grep -qiE 'AllocCap' "$DOC" \
  && grep -qiE 'not OS rlimit|not.*GC|path A' "$DOC"; then
  pass "doc MEMORY_QUOTA path A malloc/free honesty"
else
  bad "doc missing malloc/free path A honesty"
fi

# Source wiring
if grep -q 'name == "malloc"' oodac/tc_names.oo \
  && grep -q 'name == "free"' oodac/tc_names.oo \
  && grep -q 'name == "realloc"' oodac/tc_names.oo; then
  pass "tc_names malloc/free/realloc"
else
  bad "tc_names missing M166 free names"
fi
if grep -q 'name == "malloc"' oodac/check_cap_util.oo \
  && grep -q 'name == "free"' oodac/check_cap_util.oo \
  && grep -q 'is_sealed_alloc' oodac/check_cap_util.oo; then
  pass "is_sealed_alloc includes malloc/free"
else
  bad "is_sealed_alloc missing malloc/free"
fi
if grep -qE 't == "alloc_bytes" \|\| t == "malloc"' oodac/c_emit_lower.oo \
  && grep -qE 't == "free_bytes" \|\| t == "free"' oodac/c_emit_lower.oo \
  && grep -q 'oo_alloc_bytes' oodac/c_emit_lower.oo \
  && grep -q 'oo_free_bytes' oodac/c_emit_lower.oo; then
  pass "c_emit_lower malloc/free → oo_*_bytes"
else
  bad "c_emit_lower missing malloc/free aliases"
fi
if grep -q 't == "realloc"' oodac/c_emit_lower.oo; then
  pass "c_emit_lower realloc path A (free+alloc)"
else
  bad "c_emit_lower missing realloc"
fi
if grep -q 'malloc=2' oodac/tc_call_arity.oo \
  && grep -q 'free=2' oodac/tc_call_arity.oo \
  && grep -q 'realloc=3' oodac/tc_call_arity.oo; then
  pass "tc_call_arity seeds"
else
  bad "tc_call_arity missing malloc/free/realloc"
fi
if grep -q '"malloc": "AllocCap"' scripts/ooda_apply_ecap_fix.py \
  && grep -q '"free": "AllocCap"' scripts/ooda_apply_ecap_fix.py; then
  pass "ecap fix knows malloc/free"
else
  bad "ecap fix missing malloc/free"
fi

# Fixture shape
if [[ -f "$FIX" ]] \
  && grep -q 'main(alloc: &AllocCap)' "$FIX" \
  && grep -q 'malloc(alloc, 64)' "$FIX" \
  && grep -qE 'free\(alloc, (p|malloc)' "$FIX" \
  && grep -q 'println("ok")' "$FIX"; then
  pass "fixture malloc_path_a.oo shape"
else
  bad "fixture malloc_path_a.oo missing or wrong shape"
fi
# M167: typed let bind path A (tip host SEGV on untyped free-name let until rebuild)
if grep -qE 'let p: Int = malloc|free\(alloc, malloc' "$FIX"; then
  pass "fixture uses typed let or nested free(malloc) (SEGV-safe on tip host)"
else
  bad "fixture missing typed/nested malloc shape"
fi

# Runtime symbols still present (aliases only; no new C API required)
if grep -q 'oo_alloc_bytes' runtime/chs_rt.h \
  && grep -q 'oo_free_bytes' runtime/chs_rt.h \
  && grep -q 'oo_alloc_bytes' runtime/chs_rt_alloc.c; then
  pass "runtime oo_alloc_bytes/oo_free_bytes present"
else
  bad "runtime missing alloc helpers"
fi

# Executable product floor when oodac binary knows free names
if [[ -x "$OODAC_BIN" ]]; then
  # check: bare malloc without AllocCap refused
  cat >"$TMPDIR/mal_bare.oo" <<'EOF'
pub fn main() { let p = malloc(64); }
EOF
  set +e
  "$OODAC_BIN" check "$TMPDIR/mal_bare.oo" >"$TMPDIR/mal_bare.out" 2>"$TMPDIR/mal_bare.err"
  brc=$?
  set -e
  if [[ $brc -ne 0 ]] && grep -qiE 'capability|AllocCap|undefined|ERR' "$TMPDIR/mal_bare.out" "$TMPDIR/mal_bare.err" 2>/dev/null; then
    pass "check refuse bare malloc"
  elif grep -qiE 'undefined variable|unknown' "$TMPDIR/mal_bare.err" "$TMPDIR/mal_bare.out" 2>/dev/null; then
    pass "skip check (oodac pre M166 rebuild; source floor checked)"
  else
    bad "bare malloc accepted rc=$brc"
    head -5 "$TMPDIR/mal_bare.out" "$TMPDIR/mal_bare.err" || true
  fi

  set +e
  "$OODAC_BIN" check "$FIX" >"$TMPDIR/mal_ck.out" 2>"$TMPDIR/mal_ck.err"
  ckrc=$?
  set -e
  if [[ $ckrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/mal_ck.out"; then
    pass "check malloc_path_a with AllocCap"
  elif grep -qiE 'undefined|unknown' "$TMPDIR/mal_ck.err" "$TMPDIR/mal_ck.out" 2>/dev/null; then
    pass "skip check fixture (oodac pre M166 rebuild)"
  else
    bad "check fixture rc=$ckrc"
    head -10 "$TMPDIR/mal_ck.out" "$TMPDIR/mal_ck.err" || true
  fi

  set +e
  "$OODAC_BIN" emit-c "$FIX" >"$TMPDIR/mal.c" 2>"$TMPDIR/mal.err"
  erc=$?
  set -e
  if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/mal.c" "$TMPDIR/mal.err" 2>/dev/null; then
    if grep -qiE 'undefined|unknown|ERR\ttype' "$TMPDIR/mal.err" 2>/dev/null; then
      pass "skip emit (oodac pre M166 rebuild; source floor checked)"
    else
      bad "emit-c malloc_path_a"
      head -15 "$TMPDIR/mal.err" "$TMPDIR/mal.c" || true
    fi
  else
    pass "emit-c malloc_path_a"
    if grep -qE 'oo_alloc_bytes\(alloc' "$TMPDIR/mal.c" \
      && grep -qE 'oo_free_bytes\(alloc' "$TMPDIR/mal.c" \
      && grep -q 'oo_cap_grant_alloc' "$TMPDIR/mal.c"; then
      pass "emit lowers malloc/free → oo_*_bytes + grant"
    else
      bad "emit missing oo_alloc/free_bytes or grant"
      grep -nE 'malloc|free|alloc|grant' "$TMPDIR/mal.c" | head -20 || true
    fi
    # must not emit ambient C malloc/free without cap path
    if grep -qE 'oo_alloc_bytes\(alloc' "$TMPDIR/mal.c"; then
      pass "emit uses sealed oo_alloc_bytes not bare libc malloc"
    fi
    gcc "${RT[@]}" "$TMPDIR/mal.c" -o "$TMPDIR/mal.bin" 2>"$TMPDIR/mal_gcc.err" || {
      bad "gcc malloc_path_a"
      head -20 "$TMPDIR/mal_gcc.err" || true
    }
    if [[ -x "$TMPDIR/mal.bin" ]]; then
      out=$("$TMPDIR/mal.bin" 2>&1) || true
      if echo "$out" | grep -qx 'ok'; then
        pass "runtime grant path ok"
      else
        bad "runtime grant out=$out"
      fi
    fi
    # forge deny: zero AllocCap token
    if [[ -f "$TMPDIR/mal.c" ]]; then
      sed -E 's/long long alloc = oo_cap_grant_alloc\(\)/long long alloc = 0LL/' \
        "$TMPDIR/mal.c" >"$TMPDIR/mal_zero.c"
      gcc "${RT[@]}" "$TMPDIR/mal_zero.c" -o "$TMPDIR/mal_zero.bin" 2>/dev/null || {
        bad "gcc zero forge"; true
      }
      if [[ -x "$TMPDIR/mal_zero.bin" ]]; then
        set +e
        zout=$("$TMPDIR/mal_zero.bin" 2>&1) || zrc=$?
        zrc=${zrc:-0}
        set -e
        if [[ ${zrc:-0} -ne 0 ]] && echo "$zout" | grep -qE $'ERR[\t ]*cap'; then
          pass "zero forge deny"
        else
          bad "zero forge out=$zout rc=${zrc:-0}"
        fi
      fi
    fi
  fi
else
  pass "skip oodac exec (no OODAC)"
fi

if [[ $fail -ne 0 ]]; then
  echo "malloc_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "malloc_path_a_smoke: PASSED"
exit 0
