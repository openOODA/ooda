#!/usr/bin/env bash
# job: M20 pure multi input fingerprint — same tree → same input_fp (not bit-identical bins)
# in:  OODAC_BIN / oodac/oodac + scripts/oodac_pure_build.sh + gcc + sha256sum
# out: exit 0 if two pure multi builds of the same multi-module tree emit equal non-empty input_fp
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"

OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
if [[ ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then
    OODAC="$ROOT/bootstrap/seed/oodac"
  else
    echo "ERR_NO_OODAC" >&2
    exit 1
  fi
fi

MAIN="$ROOT/bootstrap/corpus/import/pass/multi_ok.oo"
if [[ ! -f "$MAIN" ]]; then
  echo "FAIL pure_build_fp: missing $MAIN" >&2
  exit 1
fi
if [[ ! -f "$ROOT/scripts/oodac_pure_build.sh" ]]; then
  echo "FAIL pure_build_fp: missing oodac_pure_build.sh" >&2
  exit 1
fi

BASE="$TMPDIR/pure_build_fp_$$"
OUT1="$BASE/bin1"
OUT2="$BASE/bin2"
FP1="$BASE/fp1"
FP2="$BASE/fp2"
LOG1="$BASE/log1"
LOG2="$BASE/log2"
mkdir -p "$BASE"
trap 'rm -rf "$BASE"' EXIT

export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-1}"
export PURE_NO_ARC="${PURE_NO_ARC:-0}"

echo "pure_build_fp_smoke: seed=$OODAC main=$MAIN"

run_build() {
  local out="$1" fp_out="$2" log="$3"
  rm -f "$out" "$fp_out"
  set +e
  (cd "$ROOT" && env -u OODA OODAC_BIN="$OODAC" PURE_BUILD_FP_OUT="$fp_out" \
    bash "$ROOT/scripts/oodac_pure_build.sh" "$MAIN" "$out") >"$log" 2>&1
  local rc=$?
  set -e
  if [[ $rc -ne 0 || ! -x "$out" ]]; then
    echo "FAIL pure_build_fp: pure multi failed (rc=$rc out=$out)" >&2
    head -40 "$log" >&2 || true
    exit 1
  fi
  if [[ ! -s "$fp_out" ]]; then
    # Fallback: parse banner from log
    if grep -qE 'pure_build: input_fp=[0-9a-f]{64}' "$log"; then
      grep -E 'pure_build: input_fp=[0-9a-f]{64}' "$log" | head -1 \
        | sed -n 's/.*input_fp=\([0-9a-f]\{64\}\).*/\1/p' >"$fp_out"
    fi
  fi
  if [[ ! -s "$fp_out" ]]; then
    echo "FAIL pure_build_fp: missing input_fp (PURE_BUILD_FP_OUT / log banner)" >&2
    head -40 "$log" >&2 || true
    exit 1
  fi
  tr -d ' \t\r\n' <"$fp_out" >"$fp_out.clean"
  mv "$fp_out.clean" "$fp_out"
  if ! grep -qE '^[0-9a-f]{64}$' "$fp_out"; then
    echo "FAIL pure_build_fp: input_fp not 64 hex: $(cat "$fp_out")" >&2
    exit 1
  fi
  if ! grep -qE 'OK_PURE_MULTI|OK_PURE' "$log" 2>/dev/null; then
    echo "WARN pure_build_fp: binary present but log missing OK_PURE*"
  fi
}

run_build "$OUT1" "$FP1" "$LOG1"
run_build "$OUT2" "$FP2" "$LOG2"

H1="$(cat "$FP1")"
H2="$(cat "$FP2")"
echo "pure_build_fp_smoke: run1 input_fp=$H1"
echo "pure_build_fp_smoke: run2 input_fp=$H2"

if [[ "$H1" != "$H2" ]]; then
  echo "FAIL pure_build_fp: same tree produced different input_fp" >&2
  echo "  run1=$H1" >&2
  echo "  run2=$H2" >&2
  exit 1
fi
echo "OK pure_build_fp: stable across two pure multi builds"

# Optional: copy tree, mutate one source, assert fingerprint changes
TREE="$BASE/tree"
mkdir -p "$TREE"
cp -a "$ROOT/bootstrap/corpus/import/pass/lib.oo" "$TREE/lib.oo"
cp -a "$ROOT/bootstrap/corpus/import/pass/multi_ok.oo" "$TREE/multi_ok.oo"
OUT3="$BASE/bin3"
FP3="$BASE/fp3"
LOG3="$BASE/log3"
set +e
(cd "$ROOT" && env -u OODA OODAC_BIN="$OODAC" PURE_BUILD_FP_OUT="$FP3" \
  bash "$ROOT/scripts/oodac_pure_build.sh" "$TREE/multi_ok.oo" "$OUT3") >"$LOG3" 2>&1
rc=$?
set -e
if [[ $rc -ne 0 || ! -s "$FP3" ]]; then
  echo "WARN pure_build_fp: mutate probe skipped (baseline build failed)"
else
  H3="$(tr -d ' \t\r\n' <"$FP3")"
  echo "// m20-fp-touch" >>"$TREE/lib.oo"
  OUT4="$BASE/bin4"
  FP4="$BASE/fp4"
  LOG4="$BASE/log4"
  set +e
  (cd "$ROOT" && env -u OODA OODAC_BIN="$OODAC" PURE_BUILD_FP_OUT="$FP4" \
    bash "$ROOT/scripts/oodac_pure_build.sh" "$TREE/multi_ok.oo" "$OUT4") >"$LOG4" 2>&1
  rc=$?
  set -e
  if [[ $rc -ne 0 || ! -s "$FP4" ]]; then
    echo "WARN pure_build_fp: mutate probe skipped (mutated build failed)"
  else
    H4="$(tr -d ' \t\r\n' <"$FP4")"
    if [[ "$H3" == "$H4" ]]; then
      echo "FAIL pure_build_fp: input_fp unchanged after source edit" >&2
      echo "  before=$H3 after=$H4" >&2
      exit 1
    fi
    echo "OK pure_build_fp: input_fp changes when a module source changes"
    echo "  before=$H3"
    echo "  after =$H4"
  fi
fi

echo "pure_build_fp_smoke: PASSED"
echo "residual: input_fp is content-only; product binaries are not claimed bit-identical"
exit 0
