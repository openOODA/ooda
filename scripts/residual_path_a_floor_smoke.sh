#!/usr/bin/env bash
# Residual DESIGN free-name path A product floor — default-deny at check
# Covers remaining PM residual leaves with real refuse rails (not doc-only).
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

# Map fixture → expected residual feature token in message
# M165: verify_human + actor_spawn promoted off residual refuse
declare -A EXPECT=(
  [residual_checkpoint_fail]=TEMPORAL
  [residual_hive_fuzz_fail]=HIVEMIND
  [residual_hot_reload_fail]=HOT_RELOAD
  [residual_shadow_fail]=SHADOW
  [residual_metamorphic_fail]=METAMORPHIC
  [residual_holo_fail]=HOLOGRAPHIC
  [residual_gpu_fail]=GPU
  [residual_bare_metal_fail]=BARE_METAL
  [residual_macro_fail]=AST_MACROS
  [residual_typestate_fail]=TYPE_STATE
  [residual_dod_fail]=DOD
  [residual_telepathic_fail]=TELEPATHIC
  [residual_lsp_fail]=NATIVE_LSP
  [residual_callgraph_fail]=CALLGRAPH
  [residual_ffigen_fail]=FFI_GEN
  [residual_lto_fail]=LTO
  [residual_toolchain_fail]=TOOLCHAINS
  [residual_playground_fail]=PLAYGROUND
  [residual_meta_vs_det_fail]=META_VS_DET
)

for base in "${!EXPECT[@]}"; do
  f="$ROOT/fixtures/${base}.oo"
  [[ -f "$f" ]] || { bad "missing $base"; continue; }
  set +e
  "$OODAC_BIN" check "$f" >"$TMPDIR/rpa_${base}.out" 2>"$TMPDIR/rpa_${base}.err"
  rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    bad "accepted residual free call $base"
  elif grep -qE $'ERR\tresidual|Residual product refuse' "$TMPDIR/rpa_${base}.out" "$TMPDIR/rpa_${base}.err" 2>/dev/null; then
    pass "refuse $base (${EXPECT[$base]})"
  else
    bad "refuse $base missing residual ERR"
    head -3 "$TMPDIR/rpa_${base}.out" "$TMPDIR/rpa_${base}.err" || true
  fi
done

# Plain program still OK
set +e
"$OODAC_BIN" check "$ROOT/fixtures/residual_path_a_ok.oo" >"$TMPDIR/rpa_ok.out" 2>"$TMPDIR/rpa_ok.err"
okrc=$?
set -e
if [[ $okrc -eq 0 ]] && grep -qE '^OK' "$TMPDIR/rpa_ok.out"; then
  pass "plain main still OK"
else
  bad "plain main broke exit=$okrc"
  cat "$TMPDIR/rpa_ok.out" "$TMPDIR/rpa_ok.err" | head -5 || true
fi

# Product CLI refuse one representative (TEMPORAL residual)
if [[ -x "$OODA" ]]; then
  set +e
  "$OODA" check "$ROOT/fixtures/residual_checkpoint_fail.oo" >"$TMPDIR/rpa_prod.out" 2>"$TMPDIR/rpa_prod.err"
  prc=$?
  set -e
  if [[ $prc -eq 0 ]]; then bad "product accepted checkpoint"
  else pass "product refuse checkpoint"; fi
fi

# json-errors codes E_RESIDUAL for one
set +e
"$OODAC_BIN" check "$ROOT/fixtures/residual_checkpoint_fail.oo" --json-errors >"$TMPDIR/rpa_json.out" 2>"$TMPDIR/rpa_json.err"
jrc=$?
set -e
if [[ $jrc -ne 0 ]] && python3 - "$TMPDIR/rpa_json.out" <<'PY'
import json,sys
raw=open(sys.argv[1]).read().strip()
# tolerate leading noise: take last JSON array line
lines=[l for l in raw.splitlines() if l.strip().startswith("[")]
v=json.loads(lines[-1] if lines else raw)
assert v and v[0].get("code")=="E_RESIDUAL", v
print("shape ok")
PY
then
  pass "json-errors E_RESIDUAL"
else
  bad "json-errors E_RESIDUAL"
  head -c 300 "$TMPDIR/rpa_json.out" || true
fi

if [[ $fail -ne 0 ]]; then
  echo "residual_path_a_floor_smoke: FAILED" >&2
  exit 1
fi
echo "residual_path_a_floor_smoke: PASSED"
exit 0
