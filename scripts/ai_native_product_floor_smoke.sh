#!/usr/bin/env bash
# AI-native systems language product floor (PM executive + 2.x agent loop)
# Proves: outline → reflect → json-errors (E_CAP fix_hint+suggested_fix) → patch replace_fn
# Residual: full AST auto-apply / telepathic compile / hive-mind (not this floor)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export OODA_SRC_ROOT="$ROOT"
export OODA="${OODA:-$ROOT/bin/ooda}"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODA" ]] || { echo "ERR_NO_OODA" >&2; exit 1; }
[[ -x "$OODAC_BIN" ]] || { echo "ERR_NO_OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

FIX="${AI_NATIVE_FIX:-$ROOT/fixtures/chs_list_string.oo}"
[[ -f "$FIX" ]] || FIX="$ROOT/bootstrap/corpus/check/pass/ok_main.oo"
CAP_F="$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo"

# --- 1) outline (token-cheap public API) ---
set +e
"$OODA" outline "$FIX" >"$TMPDIR/ai_outline.out" 2>"$TMPDIR/ai_outline.err"
orc=$?
set -e
if [[ $orc -eq 0 ]] && [[ -s "$TMPDIR/ai_outline.out" ]]; then
  pass "outline pass non-empty"
else
  bad "outline exit=$orc"
  head -5 "$TMPDIR/ai_outline.err" || true
fi

# --- 2) reflect (NDJSON agent metadata) ---
set +e
"$OODA" reflect "$FIX" >"$TMPDIR/ai_reflect.out" 2>"$TMPDIR/ai_reflect.err"
rrc=$?
set -e
if [[ $rrc -eq 0 ]] && [[ -s "$TMPDIR/ai_reflect.out" ]]; then
  pass "reflect pass non-empty"
else
  bad "reflect exit=$rrc"
fi

# --- 3) json-errors: E_CAP + fix_hint + suggested_fix (agent apply surface) ---
set +e
"$OODA" check "$CAP_F" --json-errors >"$TMPDIR/ai_je.out" 2>"$TMPDIR/ai_je.err"
jrc=$?
set -e
if [[ $jrc -eq 0 ]]; then
  bad "json-errors cap should fail"
elif python3 - "$TMPDIR/ai_je.out" <<'PY'
import json, sys
raw = open(sys.argv[1]).read().strip()
lines = [l for l in raw.splitlines() if l.strip().startswith("[")]
v = json.loads(lines[-1] if lines else raw)
assert v and v[0].get("code") == "E_CAP", v
fh = v[0].get("fix_hint") or ""
assert "cap" in fh.lower() or "capability" in fh.lower(), fh
sf = v[0].get("suggested_fix") or ""
assert len(sf) > 0 and ("cap" in sf.lower() or "NetCap" in sf or "FsCap" in sf), sf
assert v[0].get("kind") == "CapabilitySecurityViolation", v[0]
print("shape ok")
PY
then
  pass "json-errors E_CAP + fix_hint + suggested_fix"
else
  bad "json-errors E_CAP agent fields"
  head -c 400 "$TMPDIR/ai_je.out" || true
fi

# --- 4) patch replace_fn (surgical edit) ---
WORK="$ROOT/.ai_native_patch_$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT
cp "$ROOT/fixtures/patch_add.oo" "$WORK/target.oo"
cp "$ROOT/fixtures/patch_add_body.txt" "$WORK/body.txt"
REL_T="${WORK#$ROOT/}/target.oo"
REL_B="${WORK#$ROOT/}/body.txt"
set +e
"$OODA" patch "$REL_T" --replace-fn add --with "$REL_B" \
  >"$TMPDIR/ai_patch.out" 2>"$TMPDIR/ai_patch.err"
prc=$?
set -e
if [[ $prc -eq 0 ]] && grep -q 'return a - b' "$WORK/target.oo"; then
  pass "patch replace_fn applied"
else
  bad "patch replace_fn exit=$prc"
  head -10 "$TMPDIR/ai_patch.err" || true
fi

# --- 5) residual: no fake auto-apply free call ---
set +e
"$OODAC_BIN" check "$ROOT/fixtures/residual_telepathic_fail.oo" >"$TMPDIR/ai_tele.out" 2>"$TMPDIR/ai_tele.err"
trc=$?
set -e
if [[ $trc -ne 0 ]] && grep -qE $'ERR\tresidual|Residual product refuse' "$TMPDIR/ai_tele.out" "$TMPDIR/ai_tele.err" 2>/dev/null; then
  pass "telepathic free-name refuse (no fake intent compile)"
else
  bad "telepathic should residual-refuse"
fi

# --- 6) component rails still green ---
for s in outline_reflect_smoke.sh patch_smoke.sh json_errors_smoke.sh; do
  if bash "$ROOT/scripts/$s" >"$TMPDIR/ai_comp_$s.log" 2>&1; then
    pass "component $s"
  else
    bad "component $s"
    tail -8 "$TMPDIR/ai_comp_$s.log" || true
  fi
done

if [[ $fail -ne 0 ]]; then
  echo "ai_native_product_floor_smoke: FAILED" >&2
  exit 1
fi
echo "ai_native_product_floor_smoke: PASSED"
exit 0
