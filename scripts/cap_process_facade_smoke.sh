#!/usr/bin/env bash
# Process facade smoke — ProcessCap on process_exec; SysCap on process_prctl
#
# Criteria (exit 0 when path-A facade holds):
#   1. Static: std/os/process.oo process_exec formal is &ProcessCap
#   2. Static: process_prctl formal stays &SysCap (ProcessCap↛hard Sys)
#   3. Check deny: ProcessCap-only + sys_prctl (or process_prctl shape)
#   4. Check allow: ProcessCap + sys_exec
#
# Wire: after cap_g6_std_migrate_smoke in ci_product. No push.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-.ooda-cache/ooda-tmp}/cap_process_facade_smoke_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

PROC="$ROOT/std/os/process.oo"
[[ -f "$PROC" ]] || { echo "ERR: missing $PROC" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

# ---------------------------------------------------------------------------
# 1) Static: process_exec takes &ProcessCap
# ---------------------------------------------------------------------------
exec_sig="$(grep -E '^\s*pub\s+fn\s+process_exec\s*\(' "$PROC" 2>/dev/null | head -1 || true)"
if [[ -z "$exec_sig" ]]; then
  bad "no pub fn process_exec in process.oo"
elif echo "$exec_sig" | grep -qE '&ProcessCap'; then
  if echo "$exec_sig" | grep -qE '&SysCap'; then
    bad "process_exec still mentions &SysCap: $exec_sig"
  else
    pass "static process_exec uses &ProcessCap"
  fi
elif echo "$exec_sig" | grep -qE '&SysCap'; then
  bad "process_exec still SysCap-only: $exec_sig"
else
  bad "process_exec missing &ProcessCap formal: $exec_sig"
fi

# ---------------------------------------------------------------------------
# 2) Static: process_prctl stays &SysCap
# ---------------------------------------------------------------------------
prctl_sig="$(grep -E '^\s*pub\s+fn\s+process_prctl\s*\(' "$PROC" 2>/dev/null | head -1 || true)"
if [[ -z "$prctl_sig" ]]; then
  bad "no pub fn process_prctl in process.oo"
elif echo "$prctl_sig" | grep -qE '&SysCap'; then
  if echo "$prctl_sig" | grep -qE '&ProcessCap'; then
    bad "process_prctl must not take &ProcessCap: $prctl_sig"
  else
    pass "static process_prctl uses &SysCap"
  fi
else
  bad "process_prctl missing &SysCap formal: $prctl_sig"
fi

# ---------------------------------------------------------------------------
# Check helpers
# ---------------------------------------------------------------------------
check_deny() {
  local name=$1 body=$2
  printf '%s\n' "$body" >"$TMP/$name.oo"
  set +e
  out=$("$OODAC" check "$TMP/$name.oo" 2>&1); rc=$?
  set -e
  if [[ $rc -eq 0 ]] && ! echo "$out" | grep -qiE 'ERR|Capability|cap'; then
    bad "check accepted deny $name"
  else
    pass "deny $name (rc=$rc)"
  fi
}
check_allow() {
  local name=$1 body=$2
  printf '%s\n' "$body" >"$TMP/$name.oo"
  set +e
  out=$("$OODAC" check "$TMP/$name.oo" 2>&1); rc=$?
  set -e
  if [[ $rc -ne 0 ]] || echo "$out" | grep -qiE 'Capability Violation|requires a &'; then
    bad "check rejected allow $name (rc=$rc): $out"
  else
    pass "allow $name"
  fi
}

# ---------------------------------------------------------------------------
# 3) Deny: ProcessCap-only + sys_prctl (hard Sys; process_prctl shape)
# ---------------------------------------------------------------------------
check_deny process_prctl_shape \
  'fn main(p: &ProcessCap) { let _ = sys_prctl(p, 0); }'

# ---------------------------------------------------------------------------
# 4) Allow: ProcessCap + sys_exec
# ---------------------------------------------------------------------------
check_allow process_exec_sealed \
  'fn main(p: &ProcessCap) { let _ = sys_exec(p, "true"); }'

# Soft: product facade itself typechecks under dual ProcessCap + SysCap main
set +e
pout=$("$OODAC" check "$PROC" 2>&1); prc=$?
set -e
if [[ $prc -ne 0 ]]; then
  bad "check std/os/process.oo (rc=$prc): $pout"
else
  pass "check std/os/process.oo"
fi

# ---------------------------------------------------------------------------
# 5) Honesty: python_embed facade stays &SysCap (do NOT greenwash as ProcessCap)
# ---------------------------------------------------------------------------
PY_FACADES=(
  "$ROOT/std/os/python.oo"
  "${STD_ROOT:-$ROOT/../std}/python.oo"
)
py_checked=0
for f in "${PY_FACADES[@]}"; do
  [[ -f "$f" ]] || continue
  py_checked=1
  sig="$(grep -E '^\s*pub\s+fn\s+load_model\s*\(' "$f" 2>/dev/null | head -1 || true)"
  if [[ -z "$sig" ]]; then
    bad "no pub fn load_model in $f"
  elif echo "$sig" | grep -qE '&ProcessCap'; then
    bad "python load_model must stay SysCap (got ProcessCap): $sig"
  elif echo "$sig" | grep -qE '&SysCap'; then
    pass "python load_model SysCap honesty ($f)"
  else
    bad "python load_model missing &SysCap formal ($f): $sig"
  fi
done
if [[ $py_checked -eq 0 ]]; then
  note() { echo "NOTE $*"; }
  note "no python.oo facade found — SysCap honesty residual"
  pass "python honesty residual (optional path)"
fi

# Deny: ProcessCap alone must not unlock python_embed
check_deny process_python_embed \
  'fn main(p: &ProcessCap) { let _ = python_embed_internal(p, "x"); }'

# ---------------------------------------------------------------------------
# 6) Optional async split honesty (spawn ProcessCap / join SysCap)
# ---------------------------------------------------------------------------
ASYNC_FACADES=(
  "$ROOT/std/os/async.oo"
  "${STD_ROOT:-$ROOT/../std}/async.oo"
)
for f in "${ASYNC_FACADES[@]}"; do
  [[ -f "$f" ]] || continue
  spawn_sig="$(grep -E '^\s*pub\s+fn\s+spawn_task\s*\(' "$f" 2>/dev/null | head -1 || true)"
  join_sig="$(grep -E '^\s*pub\s+fn\s+join_task\s*\(' "$f" 2>/dev/null | head -1 || true)"
  if echo "$spawn_sig" | grep -qE '&ProcessCap'; then
    pass "async spawn_task uses &ProcessCap ($f)"
  else
    bad "async spawn_task missing &ProcessCap ($f): $spawn_sig"
  fi
  if echo "$join_sig" | grep -qE '&SysCap' && ! echo "$join_sig" | grep -qE '&ProcessCap'; then
    pass "async join_task stays &SysCap ($f)"
  else
    bad "async join_task must stay SysCap-only ($f): $join_sig"
  fi
done

# Soft: ci_product wire after cap_g6
CI_PRODUCT="$ROOT/scripts/ci_product.sh"
if [[ -f "$CI_PRODUCT" ]] && grep -q 'cap_process_facade_smoke' "$CI_PRODUCT" 2>/dev/null; then
  # Prefer ordering: cap_g6 then this smoke
  if grep -A2 'cap_g6_std_migrate_smoke' "$CI_PRODUCT" 2>/dev/null | grep -q 'cap_process_facade_smoke'; then
    pass "ci_product wire after cap_g6"
  else
    pass "ci_product soft-wire present"
  fi
else
  pass "ci_product soft-wire residual (optional)"
fi

if [[ $fail -ne 0 ]]; then
  echo "cap_process_facade_smoke: FAILED" >&2
  exit 1
fi
echo "cap_process_facade_smoke: PASSED"
exit 0
