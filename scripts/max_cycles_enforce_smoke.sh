#!/usr/bin/env bash
# M48 MaxCycles path A — Backend-C while fuel under // MAX_CYCLES: N
# job: emit+gcc+run pass (under N) and fail (tight N → ERR\tmax_cycles\texceeded)
# devil: inject fallback if native emit lacks fuel; zero-N fail-closed; while-only fuel
# residual: not OS cgroup; for / #[MaxCycles] not covered here
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
INJ="$ROOT/scripts/max_cycles_fuel_inject.sh"

if [[ ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then OODAC="$ROOT/bootstrap/seed/oodac"
  else echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; fi
fi
chmod +x "$INJ" 2>/dev/null || true

PASS="$ROOT/fixtures/max_cycles_pass.oo"
FAIL="$ROOT/fixtures/max_cycles_fail.oo"
ZERO="$ROOT/fixtures/max_cycles_zero.oo"
DOC="$ROOT/bootstrap/MAX_CYCLES.md"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -f "$PASS" && -f "$FAIL" ]] || { echo "ERR_NO_FIXTURE" >&2; exit 1; }

# Emit raw C; if no fuel and inject available, apply inject rail (no pure_build).
emit_fuel_c() {
  local src="$1" tag="$2"
  local raw="$TMPDIR/mc_${tag}_raw.c"
  local out="$TMPDIR/mc_${tag}.c"
  set +e
  "$OODAC" emit-c "$src" >"$raw" 2>"$TMPDIR/mc_${tag}.err"
  local rc=$?
  if [[ $rc -ne 0 || ! -s "$raw" ]] && [[ -x "$ROOT/bootstrap/seed/oodac" && "$OODAC" != "$ROOT/bootstrap/seed/oodac" ]]; then
    "$ROOT/bootstrap/seed/oodac" emit-c "$src" >"$raw" 2>"$TMPDIR/mc_${tag}.err"
    rc=$?
  fi
  set -e
  if [[ "$rc" != "0" ]]; then
    echo "FAIL emit-c exit $rc: $src" >&2
    cat "$TMPDIR/mc_${tag}.err" >&2 || true
    return 1
  fi
  if grep -E $'^ERR\t' "$raw" >/dev/null 2>&1; then
    # Native may fail-closed zero/invalid at emit — surface to caller
    cp -- "$raw" "$out"
    return 2
  fi
  grep -v '🚀\|Running main' "$raw" >"${raw}.clean" || true
  mv "${raw}.clean" "$raw"
  if grep -q '__oo_mc' "$raw"; then
    cp -- "$raw" "$out"
    echo native >"$TMPDIR/mc_${tag}.mode"
  else
    # Attack: fuel not applied — inject rail MUST land while fuel
    set +e
    bash "$INJ" "$src" "$raw" "$out" 2>"$TMPDIR/mc_${tag}_inj.err"
    local irc=$?
    set -e
    if [[ $irc -ne 0 ]]; then
      cat "$TMPDIR/mc_${tag}_inj.err" >&2 || true
      return 3
    fi
    if ! grep -qE '__oo_mc_fuel|__oo_mc' "$out"; then
      echo "FAIL fuel still missing after inject: $src" >&2
      return 4
    fi
    echo inject >"$TMPDIR/mc_${tag}.mode"
  fi
  # Attack: wrong loops — for must not carry fuel check on same construct
  if grep -E 'for[[:space:]]*\([^)]*__oo_mc' "$out" >/dev/null 2>&1; then
    echo "FAIL fuel applied to for-loop residual: $src" >&2
    return 5
  fi
  # ARC decls if needed
  if grep -q 'oo_str_release\|oo_slist_release' "$out" && ! grep -q 'void oo_str_release' "$out"; then
    awk '
      { print }
      /} OoSList;/ && !done {
        print "void oo_slist_retain(OoSList); void oo_slist_release(OoSList);"
        print "void oo_ilist_retain(OoIList); void oo_ilist_release(OoIList);"
        print "void oo_str_retain(OoStr); void oo_str_release(OoStr);"
        done = 1
      }
    ' "$out" >"${out}.arc" && mv "${out}.arc" "$out"
  fi
  printf '%s' "$out"
  return 0
}

# --- pass: loop under N ---
set +e
c_pass="$(emit_fuel_c "$PASS" pass)"
prc=$?
set -e
if [[ $prc -ne 0 || ! -f "$c_pass" ]]; then
  bad "pass emit/fuel failed rc=$prc"
else
  if grep -qE '__oo_mc' "$c_pass"; then
    pass "pass emit has while fuel ($(cat "$TMPDIR/mc_pass.mode" 2>/dev/null || echo ?))"
  else
    bad "pass emit missing fuel counter"
  fi
  if grep -qE 'max_cycles' "$c_pass"; then
    pass "pass emit has max_cycles exceed path"
  else
    bad "pass emit missing max_cycles exceed path"
  fi
  bin_pass="$TMPDIR/mc_pass.bin"
  set +e
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_pass" -o "$bin_pass" -lm \
    >"$TMPDIR/mc_pass_gcc.out" 2>"$TMPDIR/mc_pass_gcc.err"
  grc=$?
  set -e
  if [[ $grc -ne 0 || ! -x "$bin_pass" ]]; then
    bad "pass gcc failed"
    cat "$TMPDIR/mc_pass_gcc.err" >&2 || true
  else
    out_pass="$(timeout 3 "$bin_pass" 2>&1 || true)"
    if echo "$out_pass" | grep -qE $'ERR\tmax_cycles'; then
      bad "pass should not hit max_cycles: $out_pass"
    else
      echo "$out_pass" | grep -qE '3' && pass "pass run under N (got 3)" || bad "pass run expected 3 got: $out_pass"
    fi
  fi
fi

# --- fail: tight N ---
set +e
c_fail="$(emit_fuel_c "$FAIL" fail)"
frc=$?
set -e
if [[ $frc -ne 0 || ! -f "$c_fail" ]]; then
  bad "fail emit/fuel failed rc=$frc"
else
  if grep -qE '__oo_mc' "$c_fail"; then
    pass "fail emit has while fuel"
  else
    bad "fail emit missing fuel counter"
  fi
  bin_fail="$TMPDIR/mc_fail.bin"
  set +e
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$c_fail" -o "$bin_fail" -lm \
    >"$TMPDIR/mc_fail_gcc.out" 2>"$TMPDIR/mc_fail_gcc.err"
  grc=$?
  set -e
  if [[ $grc -ne 0 || ! -x "$bin_fail" ]]; then
    bad "fail gcc failed"
  else
    set +e
    out_fail="$(timeout 3 "$bin_fail" 2>&1)"
    er=$?
    set -e
    if [[ $er -eq 0 ]]; then
      bad "fail fixture should non-zero exit out=$out_fail"
    else
      pass "fail exit non-zero (rc=$er)"
    fi
    if echo "$out_fail" | grep -qE $'ERR\tmax_cycles'; then
      pass "fail prints ERR\\tmax_cycles..."
    else
      bad "fail missing ERR\\tmax_cycles out=$out_fail"
    fi
  fi
fi

# --- Attack 3: silent zero N — must fail-closed (native ERR or inject non-zero) ---
if [[ -f "$ZERO" ]]; then
  set +e
  "$OODAC" emit-c "$ZERO" >"$TMPDIR/mc_zero_raw.c" 2>"$TMPDIR/mc_zero.err"
  zrc=$?
  set -e
  zhit=0
  if grep -qE $'ERR\tmax_cycles' "$TMPDIR/mc_zero_raw.c" "$TMPDIR/mc_zero.err" 2>/dev/null; then
    zhit=1
  fi
  if [[ $zrc -ne 0 ]]; then zhit=1; fi
  if [[ $zhit -eq 0 ]]; then
    set +e
    bash "$INJ" "$ZERO" "$TMPDIR/mc_zero_raw.c" "$TMPDIR/mc_zero_fuel.c" 2>"$TMPDIR/mc_zero_inj.err"
    irc=$?
    set -e
    if [[ $irc -ne 0 ]] && grep -qiE 'zero N|fail-closed|max_cycles' "$TMPDIR/mc_zero_inj.err"; then
      zhit=1
    fi
  fi
  if [[ $zhit -eq 1 ]]; then
    pass "zero N fail-closed (not silent)"
  else
    bad "zero N silently accepted (must fail-closed)"
  fi
else
  bad "missing zero fixture"
fi

# --- honesty: residual docs not names-only after path A In ---
if [[ -f "$DOC" ]]; then
  if grep -qE 'cgroup|not OS' "$DOC"; then
    pass "doc still denies OS isolation"
  else
    bad "MAX_CYCLES.md missing OS isolation denial"
  fi
  if grep -qE '__oo_mc|while fuel|path A' "$DOC"; then
    pass "doc describes path A while fuel"
  else
    bad "MAX_CYCLES.md missing path A while fuel story"
  fi
  _nn="$(
    grep -nE 'names only|named marker only|neither is lowered' "$DOC" 2>/dev/null \
      | grep -viE '#\[MaxCycles\]|attribute|residual|was M21' || true
  )"
  if [[ -n "$_nn" ]]; then
    bad "doc still claims names-only product after path A In"
    echo "$_nn" >&2
  else
    pass "doc does not claim names-only product surface"
  fi
fi

if grep -q 'max_cycles_enforce_smoke.sh' "$ROOT/scripts/ci_product.sh"; then
  pass "ci_product wires max_cycles_enforce_smoke"
else
  bad "ci_product missing max_cycles_enforce_smoke.sh"
fi

if [[ $fail -ne 0 ]]; then
  echo "max_cycles_enforce_smoke: FAILED" >&2
  exit 1
fi
echo "max_cycles_enforce_smoke: PASSED"
exit 0
