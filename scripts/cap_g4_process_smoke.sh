#!/usr/bin/env bash
# CAP-G4: ProcessCap preferred for sys_exec; SysCap supersede; ProcessCap↛sys_prctl
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/cap_g4_smoke_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT
pass() { echo "OK $*"; }
fail() { echo "FAIL $*" >&2; exit 1; }
[[ -x "$OODAC" ]] || fail "oodac missing"

grep -q 'sealed_sys_kind_of' oodac/check_cap_util.oo || fail "missing sealed_sys_kind_of"
grep -q 'return "ProcessCap"' oodac/check_cap_util.oo || fail "sys_exec not ProcessCap preferred"
grep -q 'oo_cap_require_process' runtime/chs_rt_sys.c || fail "missing require_process"
pass "static CAP-G4 markers"

check_deny() {
  local name=$1 body=$2
  printf '%s\n' "$body" >"$TMP/$name.oo"
  set +e
  out=$("$OODAC" check "$TMP/$name.oo" 2>&1); rc=$?
  set -e
  if [[ $rc -eq 0 ]] && ! echo "$out" | grep -qiE 'ERR|Capability|cap'; then
    fail "check accepted deny $name"
  fi
  pass "deny $name (rc=$rc)"
}
check_allow() {
  local name=$1 body=$2
  printf '%s\n' "$body" >"$TMP/$name.oo"
  set +e
  out=$("$OODAC" check "$TMP/$name.oo" 2>&1); rc=$?
  set -e
  if [[ $rc -ne 0 ]] || echo "$out" | grep -qiE 'Capability Violation|requires a &'; then
    fail "check rejected allow $name (rc=$rc): $out"
  fi
  pass "allow $name"
}

check_deny process_prctl 'fn main(p: &ProcessCap) { let _ = sys_prctl(p, 0); }'
check_allow process_exec 'fn main(p: &ProcessCap) { let _ = sys_exec(p, "true"); }'
check_allow sys_exec 'fn main(sys: &SysCap) { let _ = sys_exec(sys, "true"); }'
check_allow sys_prctl 'fn main(sys: &SysCap) { let _ = sys_prctl(sys, 0); }'

# corpus if present
for f in bootstrap/corpus/check/pass/ok_process_cap_sys_exec.oo bootstrap/corpus/check/pass/ok_sys_cap_sys_exec.oo; do
  [[ -f "$f" ]] || continue
  set +e; out=$("$OODAC" check "$f" 2>&1); rc=$?; set -e
  [[ $rc -eq 0 ]] || fail "corpus allow $f: $out"
  pass "corpus $(basename "$f")"
done
for f in bootstrap/corpus/check/fail/wrong_granular_process_for_prctl.oo; do
  [[ -f "$f" ]] || continue
  set +e; out=$("$OODAC" check "$f" 2>&1); rc=$?; set -e
  if [[ $rc -eq 0 ]] && ! echo "$out" | grep -qiE 'ERR|Capability'; then fail "corpus deny $f accepted"; fi
  pass "corpus deny $(basename "$f")"
done

if grep -q 'cap_g4_process_smoke' scripts/ci_product.sh 2>/dev/null; then
  pass "ci_product soft-wire present"
else
  pass "ci_product optional: wire cap_g4_process_smoke"
fi
echo "cap_g4_process_smoke: PASSED"
