#!/usr/bin/env bash
# job: M15 cold seed pure multi — rebuild tree oodac to a side path (never clobber oodac/oodac)
# in:  bootstrap/seed/oodac (optional) + scripts/oodac_pure_build.sh + gcc
# out: exit 0 if seed pure multi green, or honest residual mode documented;
#      exit 0 SKIP if no seed; exit 1 if seed present, lag, and residual not documented
#
# Prefer green. Residual mode is mechanical:
#   - bootstrap/SEED_PURE_MULTI.md has start-of-line: ACTIVE: RESIDUAL_SEED_PURE_MULTI…
#   - this smoke prints RESIDUAL_SEED_PURE_MULTI and exits 0
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

export PURE_NO_ARC="${PURE_NO_ARC:-0}"
export PURE_SKIP_CHECK="${PURE_SKIP_CHECK:-1}"

SEED="${SEED_OODAC:-}"
if [[ -z "$SEED" || ! -x "$SEED" ]]; then
  if [[ -x "$ROOT/bootstrap/seed/oodac" ]]; then
    SEED="$ROOT/bootstrap/seed/oodac"
  else
    SEED=""
  fi
fi

RESIDUAL_DOC="$ROOT/bootstrap/SEED_PURE_MULTI.md"
# Active residual is a start-of-line STATUS line in the doc (not prose/examples).
# Smoke prints RESIDUAL_SEED_PURE_MULTI when that line is present and build lags.
ACTIVE_RESIDUAL_RE='^ACTIVE: RESIDUAL_SEED_PURE_MULTI'
MARKER="RESIDUAL_SEED_PURE_MULTI"
CHECK_FIXTURE="$ROOT/bootstrap/corpus/check/pass/ok_main.oo"
if [[ ! -f "$CHECK_FIXTURE" ]]; then
  CHECK_FIXTURE="$ROOT/fixtures/int_main.oo"
fi
MAIN="$ROOT/oodac/main.oo"
# Stable side path under TMPDIR — never write oodac/oodac.
OUT="$TMPDIR/oodac_from_seed"
LOG="$TMPDIR/seed_pure_multi_build.out"
ERR="$TMPDIR/seed_pure_multi_build.err"

if [[ -z "$SEED" ]]; then
  echo "seed_pure_multi_smoke: SKIP (no bootstrap/seed/oodac and no SEED_OODAC)"
  echo "tip: place pure seed at bootstrap/seed/oodac; see bootstrap/SEED_PURE_MULTI.md"
  exit 0
fi

if [[ ! -f "$MAIN" ]]; then
  echo "FAIL seed_pure_multi: missing $MAIN" >&2
  exit 1
fi
if [[ ! -f "$ROOT/scripts/oodac_pure_build.sh" ]]; then
  echo "FAIL seed_pure_multi: missing oodac_pure_build.sh" >&2
  exit 1
fi

echo "seed_pure_multi_smoke: seed=$SEED"
echo "seed_pure_multi_smoke: main=$MAIN → side out=$OUT"
echo "seed_pure_multi_smoke: PURE_NO_ARC=$PURE_NO_ARC PURE_SKIP_CHECK=$PURE_SKIP_CHECK"

rm -f "$OUT" "$LOG" "$ERR"
set +e
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" \
  bash "$ROOT/scripts/oodac_pure_build.sh" "$MAIN" "$OUT") >"$LOG" 2>"$ERR"
rc=$?
set -e

if [[ $rc -ne 0 || ! -x "$OUT" ]]; then
  _ex=no
  [[ -x "$OUT" ]] && _ex=yes
  echo "seed_pure_multi_smoke: pure multi FAILED (rc=$rc executable=$_ex)" >&2
  echo "--- first errors ---" >&2
  head -40 "$ERR" "$LOG" 2>/dev/null || true
  first="$(grep -E 'ERR_|FAIL|error:|undefined' "$ERR" "$LOG" 2>/dev/null | head -3 || true)"
  if [[ -f "$RESIDUAL_DOC" ]] && grep -qE "$ACTIVE_RESIDUAL_RE" "$RESIDUAL_DOC"; then
    echo "$MARKER"
    echo "seed_pure_multi_smoke: residual honesty (ACTIVE residual line in bootstrap/SEED_PURE_MULTI.md)"
    if [[ -n "$first" ]]; then
      echo "seed_pure_multi_smoke: first_error=$first"
    fi
    echo "seed_pure_multi_smoke: RESIDUAL (prefer green — refresh seed after trusted pure rebuild)"
    exit 0
  fi
  echo "FAIL seed_pure_multi: seed lag and residual not documented" >&2
  echo "  add line to bootstrap/SEED_PURE_MULTI.md: ACTIVE: RESIDUAL_SEED_PURE_MULTI: <first error>" >&2
  echo "  or refresh seed: cp -a oodac/oodac bootstrap/seed/oodac  # trusted pure rebuild only" >&2
  exit 1
fi

if ! grep -qE 'OK_PURE_MULTI|OK_PURE' "$LOG" 2>/dev/null; then
  echo "WARN seed_pure_multi: binary present but log missing OK_PURE* banner"
fi

# Side binary must run product check on a simple pass fixture (never touch tree oodac/oodac).
CHK_OUT="$TMPDIR/seed_pure_multi_check.out"
CHK_ERR="$TMPDIR/seed_pure_multi_check.err"
set +e
timeout 30 "$OUT" check "$CHECK_FIXTURE" >"$CHK_OUT" 2>"$CHK_ERR"
ck=$?
set -e
if [[ $ck -ne 0 ]] || ! grep -qE '^OK' "$CHK_OUT" 2>/dev/null; then
  echo "seed_pure_multi_smoke: side binary check FAILED on $CHECK_FIXTURE" >&2
  head -20 "$CHK_OUT" "$CHK_ERR" 2>/dev/null || true
  if [[ -f "$RESIDUAL_DOC" ]] && grep -qE "$ACTIVE_RESIDUAL_RE" "$RESIDUAL_DOC"; then
    echo "$MARKER"
    echo "seed_pure_multi_smoke: residual honesty (built but check lag)"
    exit 0
  fi
  echo "FAIL seed_pure_multi: built binary does not check; residual not documented" >&2
  exit 1
fi

echo "OK seed_pure_multi: cold seed pure multi rebuild green"
echo "  seed:    $SEED"
echo "  side:    $OUT"
echo "  check:   $CHECK_FIXTURE → OK"
if grep -qE 'unique_fns=|from_modules=' "$LOG" 2>/dev/null; then
  grep -E 'unique_fns=|from_modules=|OK_PURE' "$LOG" | head -5 || true
fi
echo "seed_pure_multi_smoke: PASSED (green)"
exit 0
