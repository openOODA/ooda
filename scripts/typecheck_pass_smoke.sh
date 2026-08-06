#!/usr/bin/env bash
# job: pure oodac check OK on bootstrap/corpus/typecheck/pass/*
# in:  OODAC_BIN (default ./oodac/oodac) + typecheck/pass corpus
# out: exit 0 if every pass fixture check succeeds (OK + exit 0)
#
# Note: typecheck/fail/* fail-closed rail lives in scripts/chs_parity.sh
#       (R1 typecheck fail-closed pure oodac) — not duplicated here.
# Cap: full pass corpus is ~sub-second; run all (no sample).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
PASS_DIR="$ROOT/bootstrap/corpus/typecheck/pass"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
n=0
shopt -s nullglob
for f in "$PASS_DIR"/*.oo; do
  [[ -f "$f" ]] || continue
  n=$((n + 1))
  base="$(basename "$f")"
  out="$TMPDIR/tc_pass_${base}.out"
  err="$TMPDIR/tc_pass_${base}.err"
  set +e
  "$OODAC" check "$f" >"$out" 2>"$err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    echo "FAIL typecheck pass exit=$rc: $base" >&2
    cat "$out" "$err" 2>/dev/null | head -12 >&2 || true
    fail=1
    continue
  fi
  if ! grep -q '^OK' "$out" 2>/dev/null; then
    echo "FAIL typecheck pass missing OK: $base" >&2
    cat "$out" "$err" 2>/dev/null | head -12 >&2 || true
    fail=1
    continue
  fi
  if grep -qE $'^ERR\t' "$out" "$err" 2>/dev/null; then
    echo "FAIL typecheck pass has ERR: $base" >&2
    fail=1
    continue
  fi
  echo "OK typecheck pass: $base"
done

if [[ "$n" -eq 0 ]]; then
  echo "ERR: no pass fixtures under $PASS_DIR" >&2
  exit 1
fi

if [[ $fail -ne 0 ]]; then
  echo "typecheck_pass_smoke: FAILED ($n fixtures)" >&2
  exit 1
fi
echo "typecheck_pass_smoke: ALL OK ($n pass fixtures)"
exit 0
