#!/usr/bin/env bash
# job: M161 Result[String,String] assert_eq / OoResS structural eq rails
# in:  oodac/oodac (OODAC_BIN), fixtures/assert_eq_result.oo
# out: exit 0 if build+run proves Ok/Err equality (path A alpha; not beta)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: $OODAC" >&2; exit 1; }

SRC="$ROOT/fixtures/assert_eq_result.oo"
BIN="$TMPDIR/assert_eq_result"
[[ -f "$SRC" ]] || { echo "ERR_MISSING $SRC" >&2; exit 1; }

# Honesty: source emit must define oo_res_eq (M161). Stale binary needs pure rebuild.
if ! grep -q 'oo_res_eq' "$ROOT/oodac/c_emit_preamble.oo" 2>/dev/null; then
  bad "oodac/c_emit_preamble.oo missing oo_res_eq source"
else
  pass "source oo_res_eq in c_emit_preamble.oo"
fi

# Probe: emit-c of fixture must carry oo_res_eq / oo_assert_eq_ress when binary is current.
set +e
"$OODAC" emit-c "$SRC" >"$TMPDIR/aer_emit.c" 2>"$TMPDIR/aer_emit.err"
erc=$?
set -e
if [[ $erc -ne 0 ]]; then
  bad "emit-c assert_eq_result exit=$erc (stale oodac? pure rebuild required)"
  head -12 "$TMPDIR/aer_emit.err" 2>/dev/null || true
elif ! grep -q 'oo_res_eq' "$TMPDIR/aer_emit.c" 2>/dev/null; then
  bad "emit-c missing oo_res_eq — oodac/oodac stale; pure rebuild required"
  echo "HINT: OODAC_BIN=\$SEED \$SEED build oodac/main.oo oodac/oodac (see scripts/bootstrap_2stage.sh)" >&2
else
  pass "emit-c includes oo_res_eq"
fi

# Build (native). Prefer direct oodac build; fall back to pure multi-module script.
rm -f "$BIN"
set +e
OODAC_BIN="$OODAC" "$OODAC" build "$SRC" "$BIN" \
  >"$TMPDIR/aer_build.out" 2>"$TMPDIR/aer_build.err"
brc=$?
set -e
if [[ $brc -ne 0 || ! -x "$BIN" ]]; then
  set +e
  OODAC_BIN="$OODAC" bash "$ROOT/scripts/oodac_pure_build.sh" "$SRC" "$BIN" \
    >"$TMPDIR/aer_pure.out" 2>"$TMPDIR/aer_pure.err"
  prc=$?
  set -e
  if [[ $prc -ne 0 || ! -x "$BIN" ]]; then
    bad "build fixtures/assert_eq_result.oo (direct+pure)"
    head -12 "$TMPDIR/aer_build.err" "$TMPDIR/aer_pure.err" 2>/dev/null || true
    if ! grep -q 'oo_res_eq' "$TMPDIR/aer_emit.c" 2>/dev/null; then
      echo "NOTE: pure rebuild of oodac may be required for M161 oo_res_eq" >&2
    fi
  else
    pass "pure build fixtures/assert_eq_result.oo"
  fi
else
  pass "build fixtures/assert_eq_result.oo"
fi

# Run: expect ok-eq / err-eq / ok-err-ne / pass (assert_eq aborts on mismatch)
if [[ -x "$BIN" ]]; then
  set +e
  "$BIN" >"$TMPDIR/aer_run.out" 2>"$TMPDIR/aer_run.err"
  rrc=$?
  set -e
  if [[ $rrc -ne 0 ]]; then
    bad "run assert_eq_result exit=$rrc"
    head -12 "$TMPDIR/aer_run.out" "$TMPDIR/aer_run.err" 2>/dev/null || true
  else
    out="$(cat "$TMPDIR/aer_run.out" 2>/dev/null || true)"
    if echo "$out" | grep -q 'ok-eq' \
      && echo "$out" | grep -q 'err-eq' \
      && echo "$out" | grep -q 'ok-err-ne' \
      && echo "$out" | grep -q 'pass' \
      && ! echo "$out" | grep -q 'should-not'; then
      pass "run assert_eq_result (Ok/Err structural eq)"
    else
      bad "run output unexpected: $out"
    fi
  fi
else
  bad "no binary to run"
fi

# Line lock (path A)
for f in assert_eq_result_smoke.sh; do
  n=$(wc -l <"$ROOT/scripts/$f")
  if [[ "$n" -gt 256 ]]; then
    bad "$f over MAX_LINES 256 ($n)"
  else
    pass "$f lines=$n (<=256)"
  fi
done
nfix=$(wc -l <"$SRC")
if [[ "$nfix" -gt 256 ]]; then
  bad "fixtures/assert_eq_result.oo over MAX_LINES 256 ($nfix)"
else
  pass "fixtures/assert_eq_result.oo lines=$nfix (<=256)"
fi

if [[ $fail -ne 0 ]]; then
  echo "assert_eq_result_smoke: FAILED" >&2
  exit 1
fi
echo "assert_eq_result_smoke: PASSED"
exit 0
