#!/usr/bin/env bash
# E_HITL // HITL: pause bounded auto-fix (M165 multi-code)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export OODA="${OODA:-$ROOT/bin/ooda}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
WORK="$TMPDIR/ehitl_fix_$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

rm -rf "$ROOT/.ooda-cache/check" 2>/dev/null || true
cp "$ROOT/fixtures/hitl_pause_fail.oo" "$WORK/h.oo"
set +e
# Deny E_HITL: no --hitl-allowed. OODA_HITL_ALLOW env is not the check gate (M-CTZ-2 CLI is).
env -u OODA_HITL_ALLOW -u OODA_HITL_AUTO_APPROVE \
  "$OODAC_BIN" check "$WORK/h.oo" --json-errors >"$WORK/before.json" 2>"$WORK/before.err"
brc=$?
set -e
# Product may print JSON on stdout with exit 0 or non-zero; require E_HITL payload.
if python3 -c 'import json,sys; raw=open(sys.argv[1]).read().strip(); lines=[l for l in raw.splitlines() if l.strip().startswith("[")]; v=json.loads(lines[-1] if lines else raw); assert v and v[0].get("code")=="E_HITL"' \
  "$WORK/before.json" 2>/dev/null; then
  pass "E_HITL before (json)"
else
  # fallback: non-json deny text
  if [[ $brc -ne 0 ]] && grep -qiE 'HITL|hitl' "$WORK/before.json" "$WORK/before.err" 2>/dev/null; then
    pass "E_HITL before (text)"
  else
    bad "expected E_HITL before"; head -5 "$WORK/before.json" "$WORK/before.err" || true
  fi
fi

set +e
python3 "$ROOT/scripts/ooda_apply_fix.py" "$WORK/h.oo" >"$WORK/apply.out" 2>"$WORK/apply.err"
arc=$?
set -e
[[ $arc -eq 0 ]] && pass "dispatcher apply exit 0" || { bad "apply failed"; cat "$WORK/apply.err" "$WORK/apply.out"; }

if grep -qE '^\s*// HITL: pause\s*$' "$WORK/h.oo"; then
  bad "pause line still present"
  cat "$WORK/h.oo"
else
  pass "exact // HITL: pause line removed"
fi

set +e
"$OODAC_BIN" check "$WORK/h.oo" --json-errors >"$WORK/after.json" 2>"$WORK/after.err"
arc2=$?
set -e
if [[ $arc2 -eq 0 ]]; then
  pass "check passes after E_HITL fix"
else
  if python3 - "$WORK/after.json" <<'PY'
import json,sys
raw=open(sys.argv[1]).read().strip()
if not raw or raw=="[]":
    raise SystemExit(0)
lines=[l for l in raw.splitlines() if l.strip().startswith("[")]
v=json.loads(lines[-1] if lines else raw)
for d in v:
    if d.get("code")=="E_HITL":
        raise SystemExit(1)
raise SystemExit(0)
PY
  then
    pass "no E_HITL after (other diags may remain)"
  else
    bad "still E_HITL after"
  fi
fi

# non-applicable: E_CAP-only must not be fixed by ehitl alone
cp "$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo" "$WORK/cap.oo"
set +e
python3 "$ROOT/scripts/ooda_apply_ehitl_fix.py" "$WORK/cap.oo" >"$WORK/na.out" 2>"$WORK/na.err"
nrc=$?
set -e
[[ $nrc -ne 0 ]] && pass "ehitl-only on E_CAP fail-closed" || bad "ehitl should not apply to E_CAP-only"

# free-form comment must not be rewritten by ehitl when no exact line
# (and when E_HITL is present only from exact pause — mid-line comment is not pause)
cat >"$WORK/safe.oo" <<'EOF'
// not a pause: HITL: pause mid-doc only
pub fn main() {
    println(0);
}
EOF
set +e
"$OODAC_BIN" check "$WORK/safe.oo" >"$WORK/safe.out" 2>"$WORK/safe.err"
src=$?
set -e
[[ $src -eq 0 ]] && pass "non-exact HITL comment does not trip check" || bad "false positive HITL"

# product CLI if present
if [[ -x "$OODA" ]]; then
  cp "$ROOT/fixtures/hitl_pause_fail.oo" "$WORK/cli.oo"
  set +e
  "$OODA" fix "$WORK/cli.oo" >"$WORK/cli.out" 2>"$WORK/cli.err"
  crc=$?
  set -e
  if [[ $crc -eq 0 ]] && ! grep -qE '^\s*// HITL: pause\s*$' "$WORK/cli.oo"; then
    pass "ooda fix E_HITL product path"
  else
    bad "ooda fix E_HITL failed"
    cat "$WORK/cli.out" "$WORK/cli.err" | head -15
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "ehitl_autofix_smoke: FAILED" >&2
  exit 1
fi
echo "ehitl_autofix_smoke: PASSED"
exit 0
