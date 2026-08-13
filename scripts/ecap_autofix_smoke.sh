#!/usr/bin/env bash
# E_CAP bounded auto-fix floor — apply structural fix; re-check no E_CAP
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export OODA="${OODA:-$ROOT/bin/ooda}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

WORK="$TMPDIR/ecap_fix_$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT

# --- happy: no_cap_fetch → fix → no E_CAP ---
cp "$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo" "$WORK/fetch.oo"
set +e
"$OODAC_BIN" check "$WORK/fetch.oo" --json-errors >"$WORK/before.json" 2>"$WORK/before.err"
brc=$?
set -e
if [[ $brc -eq 0 ]]; then bad "expected E_CAP before fix"; else pass "E_CAP before fix"; fi
if ! grep -q E_CAP "$WORK/before.json" 2>/dev/null; then
  # try stderr
  if ! grep -q E_CAP "$WORK/before.err" 2>/dev/null && ! python3 -c 'import json; v=json.load(open("'"$WORK"'/before.json")); assert v[0]["code"]=="E_CAP"' 2>/dev/null; then
    bad "before missing E_CAP in JSON"
  else
    pass "before has E_CAP"
  fi
else
  pass "before has E_CAP"
fi

set +e
bash "$ROOT/scripts/ooda_apply_ecap_fix.sh" "$WORK/fetch.oo" >"$WORK/apply.out" 2>"$WORK/apply.err"
arc=$?
set -e
if [[ $arc -ne 0 ]]; then
  bad "apply failed"
  cat "$WORK/apply.err" | head -20
else
  pass "apply exit 0"
fi

set +e
"$OODAC_BIN" check "$WORK/fetch.oo" --json-errors >"$WORK/after.json" 2>"$WORK/after.err"
arc2=$?
set -e
# After fix: either full OK or no E_CAP in JSON
if python3 - "$WORK/after.json" "$WORK/after.err" <<'PY'
import json,sys
raw=open(sys.argv[1]).read().strip()
err=open(sys.argv[2]).read()
if not raw or raw=="[]":
    raise SystemExit(0)  # clean
# may be invalid if empty
try:
    lines=[l for l in raw.splitlines() if l.strip().startswith("[")]
    v=json.loads(lines[-1] if lines else raw)
except Exception:
    # non-json fail output — check no E_CAP text
    if "E_CAP" in raw or "E_CAP" in err:
        raise SystemExit(1)
    raise SystemExit(0)
for d in v:
    if d.get("code")=="E_CAP":
        raise SystemExit(1)
raise SystemExit(0)
PY
then
  pass "after fix: no E_CAP"
else
  bad "after fix still E_CAP"
  cat "$WORK/fetch.oo"
  head -c 300 "$WORK/after.json"
fi

# structural proof
if grep -q 'NetCap' "$WORK/fetch.oo" && grep -qE 'fetch\s*\(\s*net\s*,' "$WORK/fetch.oo"; then
  pass "structural NetCap param + fetch(net,"
else
  bad "missing structural fix shape"
  cat "$WORK/fetch.oo"
fi

# --- fail-closed: parse fail is not E_CAP/E_TC-undefined apply class ---
cp "$ROOT/bootstrap/corpus/parse/fail/missing_brace.oo" "$WORK/parse.oo"
set +e
bash "$ROOT/scripts/ooda_apply_ecap_fix.sh" "$WORK/parse.oo" >"$WORK/na.out" 2>"$WORK/na.err"
nrc=$?
set -e
if [[ $nrc -ne 0 ]]; then
  pass "non-applicable parse fail-closed"
else
  bad "parse should not auto-fix"
fi

# --- product CLI: non-NetCap E_CAP (FsCap / read_file) must succeed ---
if [[ ! -x "$OODA" ]]; then
  bad "need product OODA for ooda fix rail"
else
  cp "$ROOT/bootstrap/corpus/check/fail/no_cap_read_file.oo" "$WORK/read.oo"
  set +e
  "$OODA" fix "$WORK/read.oo" >"$WORK/cli.out" 2>"$WORK/cli.err"
  crc=$?
  set -e
  if [[ $crc -ne 0 ]]; then
    bad "ooda fix read_file exit=$crc"
    cat "$WORK/cli.out" "$WORK/cli.err" | head -20 || true
  elif ! grep -qE 'OK\tfix|applied E_CAP' "$WORK/cli.out" "$WORK/cli.err" 2>/dev/null; then
    bad "ooda fix missing OK fix banner"
  elif ! grep -q 'FsCap' "$WORK/read.oo" || ! grep -qE 'read_file\s*\(\s*fs\s*,' "$WORK/read.oo"; then
    bad "ooda fix did not add FsCap + read_file(fs,"
    cat "$WORK/read.oo"
  else
    pass "ooda fix FsCap/read_file structural"
  fi
  set +e
  "$OODAC_BIN" check "$WORK/read.oo" --json-errors >"$WORK/read_after.json" 2>"$WORK/read_after.err"
  set -e
  if python3 - "$WORK/read_after.json" <<'PY'
import json,sys
raw=open(sys.argv[1]).read().strip()
if not raw or raw=="[]":
    raise SystemExit(0)
lines=[l for l in raw.splitlines() if l.strip().startswith("[")]
v=json.loads(lines[-1] if lines else raw)
for d in v:
    if d.get("code")=="E_CAP":
        raise SystemExit(1)
raise SystemExit(0)
PY
  then
    pass "ooda fix read_file: no E_CAP after"
  else
    bad "ooda fix read_file still E_CAP"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "ecap_autofix_smoke: FAILED" >&2
  exit 1
fi
echo "ecap_autofix_smoke: PASSED"
exit 0
