#!/usr/bin/env bash
# E_TC undefined-var bounded auto-fix (Phase 1 / M158)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export OODA="${OODA:-$ROOT/bin/ooda}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
WORK="$TMPDIR/etc_fix_$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

cp "$ROOT/bootstrap/corpus/typecheck/fail/undefined_var.oo" "$WORK/u.oo"
set +e
"$OODAC_BIN" check "$WORK/u.oo" --json-errors >"$WORK/before.json" 2>"$WORK/before.err"
brc=$?
set -e
[[ $brc -ne 0 ]] && pass "E_TC before" || bad "expected fail before"
python3 -c 'import json; v=json.load(open("'"$WORK"'/before.json")); assert v[0]["code"]=="E_TC"' && pass "before E_TC code" || bad "before not E_TC"

set +e
bash "$ROOT/scripts/ooda_apply_ecap_fix.sh" "$WORK/u.oo" >"$WORK/apply.out" 2>"$WORK/apply.err"
arc=$?
set -e
[[ $arc -eq 0 ]] && pass "apply exit 0" || { bad "apply failed"; cat "$WORK/apply.err"; }

if grep -q 'let no_such_var = 0' "$WORK/u.oo"; then
  pass "structural let inserted"
else
  bad "missing let"
  cat "$WORK/u.oo"
fi

set +e
"$OODAC_BIN" check "$WORK/u.oo" --json-errors >"$WORK/after.json" 2>"$WORK/after.err"
set -e
if python3 - "$WORK/after.json" <<'PY'
import json,sys,re
raw=open(sys.argv[1]).read().strip()
if not raw or raw=="[]":
    raise SystemExit(0)
lines=[l for l in raw.splitlines() if l.strip().startswith("[")]
v=json.loads(lines[-1] if lines else raw)
for d in v:
    if d.get("code")=="E_TC" and "no_such_var" in (d.get("msg") or ""):
        raise SystemExit(1)
raise SystemExit(0)
PY
then
  pass "no E_TC for no_such_var after"
else
  bad "still E_TC for no_such_var"
fi

# non-applicable: pure E_CAP without TC
cp "$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo" "$WORK/cap.oo"
# force only E_CAP path via etc script alone should fail when we call etc directly...
# dispatcher will apply E_CAP which is OK — use etc script alone for non-applicable
set +e
python3 "$ROOT/scripts/ooda_apply_etc_fix.py" "$WORK/cap.oo" >"$WORK/na.out" 2>"$WORK/na.err"
nrc=$?
set -e
[[ $nrc -ne 0 ]] && pass "etc-only on E_CAP fail-closed" || bad "etc should not apply to E_CAP-only"

# product CLI
if [[ -x "$OODA" ]]; then
  cp "$ROOT/bootstrap/corpus/typecheck/fail/undefined_var.oo" "$WORK/cli.oo"
  set +e
  "$OODA" fix "$WORK/cli.oo" >"$WORK/cli.out" 2>"$WORK/cli.err"
  crc=$?
  set -e
  if [[ $crc -eq 0 ]] && grep -q 'let no_such_var = 0' "$WORK/cli.oo"; then
    pass "ooda fix E_TC product path"
  else
    bad "ooda fix E_TC failed"
    cat "$WORK/cli.out" "$WORK/cli.err" | head -15
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "etc_autofix_smoke: FAILED" >&2
  exit 1
fi
echo "etc_autofix_smoke: PASSED"
exit 0
