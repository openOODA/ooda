#!/usr/bin/env bash
# job: --json-errors shape rail (pass empty array + fail codes)
# in:  bin/ooda + oodac/oodac pure path
# out: exit 0 if JSON diags match bootstrap/DIAG_CODES.md
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: $OODA" >&2
  exit 1
fi
if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

PASS_F="$ROOT/bootstrap/corpus/check/pass/ok_main.oo"
CAP_F="$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo"
UNDEF_F="$ROOT/bootstrap/corpus/typecheck/fail/undefined_var.oo"

# --- pass: empty errors array ---
set +e
"$OODAC" check "$PASS_F" --json-errors >"$TMPDIR/je_pass.out" 2>"$TMPDIR/je_pass.err"
rp=$?
set -e
if [[ $rp -ne 0 ]]; then
  bad "oodac pass exit=$rp"
  cat "$TMPDIR/je_pass.out" "$TMPDIR/je_pass.err" | head -20 || true
else
  if python3 - "$TMPDIR/je_pass.out" <<'PY'
import json, sys
raw = open(sys.argv[1]).read().strip()
v = json.loads(raw)
assert isinstance(v, list), v
assert v == [], v
print("shape ok")
PY
  then
    pass "oodac check --json-errors pass → []"
  else
    bad "oodac pass JSON shape: $(head -c 200 "$TMPDIR/je_pass.out")"
  fi
fi

# --- fail: capability ---
set +e
"$OODAC" check "$CAP_F" --json-errors >"$TMPDIR/je_cap.out" 2>"$TMPDIR/je_cap.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "oodac cap should fail"
else
  if python3 - "$TMPDIR/je_cap.out" "$CAP_F" <<'PY'
import json, sys
raw = open(sys.argv[1]).read().strip()
path = sys.argv[2]
v = json.loads(raw)
assert isinstance(v, list) and len(v) >= 1, v
d = v[0]
assert d.get("code") == "E_CAP", d
assert isinstance(d.get("line"), int) and d["line"] >= 1, d
assert isinstance(d.get("col"), int) and d["col"] >= 0, d
assert isinstance(d.get("msg"), str) and len(d["msg"]) > 0, d
assert "path" in d and isinstance(d["path"], str), d
assert "fetch" in d["msg"] or "NetCap" in d["msg"] or "Capability" in d["msg"], d
print("shape ok")
PY
  then
    pass "oodac check --json-errors cap → E_CAP"
  else
    bad "oodac cap JSON: $(head -c 300 "$TMPDIR/je_cap.out")"
  fi
fi

# --- fail: undefined var ---
set +e
"$OODAC" check "$UNDEF_F" --json-errors >"$TMPDIR/je_u.out" 2>"$TMPDIR/je_u.err"
ru=$?
set -e
if [[ $ru -eq 0 ]]; then
  bad "oodac undef should fail"
else
  if python3 - "$TMPDIR/je_u.out" <<'PY'
import json, sys
raw = open(sys.argv[1]).read().strip()
v = json.loads(raw)
assert isinstance(v, list) and len(v) >= 1, v
d = v[0]
assert d.get("code") == "E_TC", d
assert isinstance(d.get("line"), int) and d["line"] >= 1, d
assert "undefined" in d.get("msg", "").lower() or "no_such" in d.get("msg", ""), d
print("shape ok")
PY
  then
    pass "oodac check --json-errors undef → E_TC"
  else
    bad "oodac undef JSON: $(head -c 300 "$TMPDIR/je_u.out")"
  fi
fi

# --- product CLI forwards ---
set +e
"$OODA" check "$PASS_F" --json-errors >"$TMPDIR/je_prod.out" 2>"$TMPDIR/je_prod.err"
rprod=$?
set -e
if [[ $rprod -ne 0 ]]; then
  bad "ooda check --json-errors pass exit=$rprod"
else
  if python3 -c 'import json,sys; v=json.loads(open(sys.argv[1]).read()); assert v==[]' "$TMPDIR/je_prod.out"; then
    pass "ooda check --json-errors forwards"
  else
    bad "ooda product JSON: $(head -c 200 "$TMPDIR/je_prod.out")"
  fi
fi

set +e
"$OODA" check "$CAP_F" --json-errors >"$TMPDIR/je_prod_cap.out" 2>"$TMPDIR/je_prod_cap.err"
rpc=$?
set -e
if [[ $rpc -eq 0 ]]; then
  bad "ooda cap should fail"
else
  if python3 -c 'import json,sys; v=json.loads(open(sys.argv[1]).read()); assert v[0]["code"]=="E_CAP"' "$TMPDIR/je_prod_cap.out"; then
    pass "ooda check --json-errors cap E_CAP"
  else
    bad "ooda cap JSON: $(head -c 300 "$TMPDIR/je_prod_cap.out")"
  fi
fi

# --- -json alias on oodac ---
set +e
"$OODAC" check "$CAP_F" -json >"$TMPDIR/je_alias.out" 2>"$TMPDIR/je_alias.err"
ra=$?
set -e
if [[ $ra -eq 0 ]]; then
  bad "-json should fail on cap"
else
  if python3 -c 'import json,sys; v=json.loads(open(sys.argv[1]).read()); assert v[0]["code"]=="E_CAP"' "$TMPDIR/je_alias.out"; then
    pass "oodac -json alias"
  else
    bad "oodac -json: $(head -c 200 "$TMPDIR/je_alias.out")"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "json_errors_smoke: FAIL" >&2
  exit 1
fi
echo "json_errors_smoke: OK"
exit 0
