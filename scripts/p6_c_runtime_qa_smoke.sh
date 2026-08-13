#!/usr/bin/env bash
# P6: C-runtime adversarial floor — static + live (T3 family).
# Not a full fuzzer. Fail-closed markers + exec env + no system(3).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

SYS="$ROOT/runtime/chs_rt_sys.c"
FFI="$ROOT/runtime/chs_rt_ffi.c"
FS="$ROOT/runtime/chs_rt_fs.c"
for f in "$SYS" "$FFI" "$FS"; do
  [[ -f "$f" ]] || bad "missing $(basename "$f")"
done

if grep -q 'execvp' "$SYS" && ! grep -vE '^\s*/\*|^\s*\*' "$SYS" | grep -qE '[^a-z_]system\s*\('; then
  pass "no system(3) call; execvp present"
else
  bad "sys_exec must be execvp, not system(3)"
fi

if grep -q 'clearenv' "$SYS" && grep -qE 'OODA_|OO_' "$SYS"; then
  pass "exec env filter markers (clearenv + OODA_/OO_)"
else
  bad "missing exec env filter markers"
fi

if grep -qE 'dlopen|dlsym' "$FFI"; then
  pass "FFI surface present"
else
  bad "FFI file missing dlopen/dlsym"
fi

if grep -q 'OODA_FS_WRITEDIR' "$FS"; then
  pass "FS writedir fail-closed marker"
else
  bad "missing OODA_FS_WRITEDIR"
fi

# Live T3 smoke if present
if [[ -x "$ROOT/scripts/runtime_zt_t3_smoke.sh" ]]; then
  set +e
  timeout 60 bash "$ROOT/scripts/runtime_zt_t3_smoke.sh" >"$TMPDIR/p6_t3.out" 2>"$TMPDIR/p6_t3.err"
  trc=$?
  set -e
  if [[ $trc -eq 0 ]]; then
    pass "runtime_zt_t3_smoke exit 0"
  else
    bad "runtime_zt_t3_smoke rc=$trc"
    tail -8 "$TMPDIR/p6_t3.out" "$TMPDIR/p6_t3.err" 2>/dev/null >&2 || true
  fi
else
  bad "missing runtime_zt_t3_smoke.sh"
fi

if [[ $fail -ne 0 ]]; then
  echo "p6_c_runtime_qa_smoke: FAILED" >&2
  exit 1
fi
echo "p6_c_runtime_qa_smoke: ALL OK"
echo "RESIDUAL: not a full memory fuzzer / ASAN campaign — adversarial wave later"
exit 0
