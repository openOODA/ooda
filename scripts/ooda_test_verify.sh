#!/usr/bin/env bash
# job: pure-path ooda test — typecheck then run verify/assert_eq via Backend-C harness
# in:  $1 = source .oo; OODAC_BIN or ./oodac/oodac; optional OODA_TEST_KEEP=1
# out: exit 0 if check + all assert_eq pass; non-zero on fail (fail-closed)
# residual: --fuzz; contracts not enforced at runtime; only assert_eq! in verify bodies
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

SRC="${1:-}"
if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "ERR	test	missing source file" >&2
  exit 2
fi
if [[ "$SRC" != /* ]]; then
  SRC="$(pwd)/$SRC"
fi

OODAC="${OODAC_BIN:-}"
if [[ -z "$OODAC" || ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then
    OODAC="$ROOT/oodac/oodac"
  elif [[ -x ./oodac/oodac ]]; then
    OODAC=./oodac/oodac
  else
    echo "ERR_NO_OODAC" >&2
    exit 1
  fi
fi

# --- 1) typecheck (fail-closed) ---
set +e
"$OODAC" check "$SRC" >"$TMPDIR/ooda_test_check.out" 2>"$TMPDIR/ooda_test_check.err"
ck=$?
set -e
if [[ $ck -ne 0 ]]; then
  cat "$TMPDIR/ooda_test_check.out" 2>/dev/null || true
  cat "$TMPDIR/ooda_test_check.err" >&2 2>/dev/null || true
  echo "ERR	test	check failed" >&2
  exit "$ck"
fi

# --- 2) lower verify blocks → harness .oo ---
HARNESS="$TMPDIR/ooda_test_$$_harness.oo"
OUTBIN="$TMPDIR/ooda_test_$$_bin"
export OODA_TEST_SRC="$SRC"
export OODA_TEST_HARNESS="$HARNESS"
PY="$ROOT/scripts/ooda_test_harness.py"
if [[ ! -f "$PY" ]]; then
  echo "ERR	test	missing $PY" >&2
  exit 1
fi
set +e
python3 "$PY"
py=$?
set -e
if [[ $py -ne 0 ]]; then
  exit "$py"
fi

# No verify blocks → check-only success (empty harness)
if [[ ! -s "$HARNESS" ]]; then
  exit 0
fi

# --- 3) build harness ---
set +e
OODAC_BIN="$OODAC" "$OODAC" build "$HARNESS" "$OUTBIN" \
  >"$TMPDIR/ooda_test_build.out" 2>"$TMPDIR/ooda_test_build.err"
br=$?
set -e
if [[ $br -ne 0 || ! -x "$OUTBIN" ]]; then
  cat "$TMPDIR/ooda_test_build.out" 2>/dev/null || true
  cat "$TMPDIR/ooda_test_build.err" >&2 2>/dev/null || true
  echo "ERR	test	harness build failed" >&2
  exit 1
fi

# --- 4) run harness ---
set +e
"$OUTBIN" >"$TMPDIR/ooda_test_run.out" 2>"$TMPDIR/ooda_test_run.err"
rr=$?
set -e
cat "$TMPDIR/ooda_test_run.out" 2>/dev/null || true
if [[ $rr -ne 0 ]]; then
  cat "$TMPDIR/ooda_test_run.err" >&2 2>/dev/null || true
  echo "ERR	test	verify failed (exit=$rr)" >&2
  if [[ -z "${OODA_TEST_KEEP:-}" ]]; then
    rm -f "$HARNESS" "$OUTBIN" "$HARNESS.c" 2>/dev/null || true
  fi
  exit 1
fi

if ! grep -q "OK verify" "$TMPDIR/ooda_test_run.out"; then
  echo "ERR	test	harness ran but missing OK verify" >&2
  exit 1
fi

if [[ -z "${OODA_TEST_KEEP:-}" ]]; then
  rm -f "$HARNESS" "$OUTBIN" "$HARNESS.c" 2>/dev/null || true
fi
exit 0
