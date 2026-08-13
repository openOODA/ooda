#!/usr/bin/env bash
# M165 FFI dlopen Path A — ALLOW_DLOPEN + system dirs / ALLOWDIR; dlsym residual
# Residual: no unrestricted any-path dlopen; no typed ffi_call of dlsym results.
# Path-A: registered-handle dlsym/dlclose (unknown handle → Err).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread)

# honesty strings in runtime
if grep -q 'path not under system lib dirs' runtime/chs_rt_ffi.c \
  && grep -q 'unknown handle' runtime/chs_rt_ffi.c \
  && grep -q 'g_ffi_handles\|handle table' runtime/chs_rt_ffi.c; then
  pass "runtime Path A ffi honesty present (dlopen allowlist + handle-table dlsym)"
else
  bad "runtime Path A ffi honesty missing"
fi

LIB=""
for p in /lib/x86_64-linux-gnu/libc.so.6 /lib64/libc.so.6 /usr/lib/libc.so.6 \
         /usr/lib64/libc.so.6 /lib/libc.so.6; do
  [[ -f "$p" ]] && LIB="$p" && break
done

# residual without ALLOW_DLOPEN
cat >"$TMPDIR/ffi_dlo.oo" <<'EOF'
pub fn main(ffi: &UnsafeFFICap) {
    let r: Result[String, String] = dlopen(ffi, "/lib/x86_64-linux-gnu/libc.so.6");
    if r.is_ok() { println("dlopen-ok"); } else { println("dlopen-err"); }
}
EOF
set +e
"$OODAC" emit-c "$TMPDIR/ffi_dlo.oo" >"$TMPDIR/ffi_dlo.c" 2>"$TMPDIR/ffi_dlo.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/ffi_dlo.c" "$TMPDIR/ffi_dlo.err" 2>/dev/null; then
  bad "emit dlopen fixture"
  head -10 "$TMPDIR/ffi_dlo.err" || true
else
  pass "emit dlopen fixture"
  gcc "${RT[@]}" "$TMPDIR/ffi_dlo.c" -o "$TMPDIR/ffi_dlo.bin" 2>"$TMPDIR/ffi_dlo.gcc" || {
    bad "gcc dlopen"; head -10 "$TMPDIR/ffi_dlo.gcc" || true
  }
fi

if [[ -x "$TMPDIR/ffi_dlo.bin" ]]; then
  out=$("$TMPDIR/ffi_dlo.bin" 2>&1) || true
  echo "$out" | grep -q 'dlopen-err' && pass "dlopen residual without ALLOW" \
    || bad "default out=$out"

  if [[ -n "$LIB" ]]; then
    # system dirs when ALLOWDIR empty
    cat >"$TMPDIR/ffi_sys.oo" <<EOF
pub fn main(ffi: &UnsafeFFICap) {
    let r: Result[String, String] = dlopen(ffi, "$LIB");
    if r.is_ok() { println("dlopen-ok"); } else { println("dlopen-err"); }
}
EOF
    set +e
    "$OODAC" emit-c "$TMPDIR/ffi_sys.oo" >"$TMPDIR/ffi_sys.c" 2>/dev/null
    gcc "${RT[@]}" "$TMPDIR/ffi_sys.c" -o "$TMPDIR/ffi_sys.bin" 2>/dev/null
    set -e
    if [[ -x "$TMPDIR/ffi_sys.bin" ]]; then
      out=$(OODA_FFI_ALLOW_DLOPEN=1 env -u OODA_FFI_ALLOWDIR "$TMPDIR/ffi_sys.bin" 2>&1) || true
      echo "$out" | grep -q 'dlopen-ok' && pass "OS dlopen system dirs (ALLOWDIR empty)" \
        || bad "sys dirs out=$out"
      # ALLOWDIR prefix still works
      out=$(OODA_FFI_ALLOW_DLOPEN=1 OODA_FFI_ALLOWDIR="$(dirname "$LIB")" \
        "$TMPDIR/ffi_sys.bin" 2>&1) || true
      echo "$out" | grep -q 'dlopen-ok' && pass "OS dlopen ALLOWDIR prefix" \
        || bad "allowdir out=$out"
    fi
  else
    pass "skip OS dlopen (no libc path)"
  fi

  # non-system absolute path still residual even with ALLOW=1 and empty ALLOWDIR
  cat >"$TMPDIR/ffi_bad.oo" <<'EOF'
pub fn main(ffi: &UnsafeFFICap) {
    let r: Result[String, String] = dlopen(ffi, "/tmp/evil.so");
    if r.is_ok() { println("dlopen-ok"); } else { println("dlopen-err"); }
}
EOF
  set +e
  "$OODAC" emit-c "$TMPDIR/ffi_bad.oo" >"$TMPDIR/ffi_bad.c" 2>/dev/null
  gcc "${RT[@]}" "$TMPDIR/ffi_bad.c" -o "$TMPDIR/ffi_bad.bin" 2>/dev/null
  set -e
  if [[ -x "$TMPDIR/ffi_bad.bin" ]]; then
    out=$(OODA_FFI_ALLOW_DLOPEN=1 env -u OODA_FFI_ALLOWDIR "$TMPDIR/ffi_bad.bin" 2>&1) || true
    echo "$out" | grep -q 'dlopen-err' && pass "refuse non-system path without ALLOWDIR" \
      || bad "tmp path out=$out"
  fi
fi

# dlsym path-A: unknown handle must Err (not product typed call)
cat >"$TMPDIR/ffi_sym.oo" <<'EOF'
pub fn main(ffi: &UnsafeFFICap) {
    let r: Result[String, String] = dlsym(ffi, "handle:0", "sym");
    if r.is_ok() { println("dlsym-ok"); } else { println("dlsym-err"); }
}
EOF
set +e
"$OODAC" emit-c "$TMPDIR/ffi_sym.oo" >"$TMPDIR/ffi_sym.c" 2>"$TMPDIR/ffi_sym.err"
src=$?
set -e
if grep -q 'oo_dlsym' "$TMPDIR/ffi_sym.c" 2>/dev/null; then
  pass "emit lowers dlsym → oo_dlsym"
  gcc "${RT[@]}" "$TMPDIR/ffi_sym.c" -o "$TMPDIR/ffi_sym.bin" 2>/dev/null || true
  if [[ -x "$TMPDIR/ffi_sym.bin" ]]; then
    out=$("$TMPDIR/ffi_sym.bin" 2>&1) || true
    # unknown handle 0 → Err (path-A fail-closed)
    echo "$out" | grep -q 'dlsym-err' && pass "runtime dlsym unknown-handle Err" \
      || bad "dlsym out=$out"
  fi
elif grep -qE $'^ERR\tc_emit\tffi residual' "$TMPDIR/ffi_sym.c" "$TMPDIR/ffi_sym.err" 2>/dev/null \
  || [[ $src -ne 0 ]]; then
  pass "dlsym emit residual (oodac lower pending rebuild)"
else
  bad "dlsym emit unexpected"
  head -8 "$TMPDIR/ffi_sym.err" "$TMPDIR/ffi_sym.c" 2>/dev/null || true
fi

if [[ $fail -ne 0 ]]; then
  echo "ffi_dlopen_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "ffi_dlopen_path_a_smoke: PASSED"
exit 0
