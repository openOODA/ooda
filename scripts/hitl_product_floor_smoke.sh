#!/usr/bin/env bash
# HITL non-interactive deny-mode product floor (M157)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
rm -rf "$ROOT/.ooda-cache/check" 2>/dev/null || true
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export OODA="${OODA:-$ROOT/bin/ooda}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

set +e
"$OODAC_BIN" check "$ROOT/fixtures/hitl_pause_fail.oo" >"$TMPDIR/hitl_f.out" 2>"$TMPDIR/hitl_f.err"
frc=$?
set -e
if [[ $frc -ne 0 ]] && grep -qE $'ERR\thitl|non-interactive deny' "$TMPDIR/hitl_f.out" "$TMPDIR/hitl_f.err" 2>/dev/null; then
  pass "check deny // HITL: pause"
else
  bad "pause marker should fail-closed"
  cat "$TMPDIR/hitl_f.out" "$TMPDIR/hitl_f.err" | head -10
fi

set +e
"$OODAC_BIN" check "$ROOT/fixtures/hitl_pause_pass.oo" >"$TMPDIR/hitl_p.out" 2>"$TMPDIR/hitl_p.err"
prc=$?
set -e
if [[ $prc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/hitl_p.out"; then
  pass "check allow without pause marker"
else
  bad "clean file should pass"
fi

# json-errors E_HITL
set +e
"$OODAC_BIN" check "$ROOT/fixtures/hitl_pause_fail.oo" --json-errors >"$TMPDIR/hitl_j.out" 2>"$TMPDIR/hitl_j.err"
jrc=$?
set -e
if [[ $jrc -ne 0 ]] && python3 - "$TMPDIR/hitl_j.out" <<'PY'
import json,sys
raw=open(sys.argv[1]).read().strip()
lines=[l for l in raw.splitlines() if l.strip().startswith("[")]
v=json.loads(lines[-1] if lines else raw)
assert v and v[0].get("code")=="E_HITL", v
print("ok")
PY
then
  pass "json-errors E_HITL"
else
  bad "E_HITL json"
  head -c 300 "$TMPDIR/hitl_j.out" || true
fi

# M165: verify_human is product free builtin (not residual refuse)
set +e
"$OODAC_BIN" check "$ROOT/fixtures/hitl_verify_human.oo" >"$TMPDIR/hitl_vh.out" 2>"$TMPDIR/hitl_vh.err"
vrc=$?
set -e
if [[ $vrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/hitl_vh.out"; then
  pass "verify_human free builtin check OK"
else
  bad "verify_human should check OK (path A product)"
  head -8 "$TMPDIR/hitl_vh.out" "$TMPDIR/hitl_vh.err" || true
fi

# residual honesty pack still green (interactive harness residual)
if bash "$ROOT/scripts/hitl_residual_smoke.sh" >"$TMPDIR/hitl_res.log" 2>&1; then
  pass "hitl_residual_smoke"
else
  bad "hitl_residual_smoke"
  tail -15 "$TMPDIR/hitl_res.log" || true
fi

if [[ $fail -ne 0 ]]; then
  echo "hitl_product_floor_smoke: FAILED" >&2
  exit 1
fi
echo "hitl_product_floor_smoke: PASSED"
exit 0
