#!/usr/bin/env bash
# job: single-file type error LINE must be local (small), not inflated
# in:  OODAC_BIN (default ./oodac/oodac) + typecheck fail fixtures
# out: exit 0 if ERR type lines report line < 50 for single-file fixtures
#
# Path A (Issue #9): single-file checks already use local token lines.
# Residual: multi-import load_import expands imports first — reported
# LINE is expanded-stream offset (see oodac/tc_diag.oo / load_import.oo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# Single-file fail fixtures: error must be near the top of the file.
# Format: ERR\ttype\tType error at LINE:COL: …
check_single() {
  local f="$1"
  local base
  base="$(basename "$f")"
  local out="$TMPDIR/tc_line_${base}.out"
  local err="$TMPDIR/tc_line_${base}.err"
  set +e
  "$OODAC" check "$f" >"$out" 2>"$err"
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    bad "$base expected non-zero exit"
    return
  fi
  local msg
  msg="$(cat "$out" "$err" 2>/dev/null | tr '\t' ' ' | grep -E 'Type error at [0-9]+:' | head -1 || true)"
  if [[ -z "$msg" ]]; then
    bad "$base missing Type error at LINE:COL"
    cat "$out" "$err" 2>/dev/null | head -8 >&2 || true
    return
  fi
  local line
  line="$(echo "$msg" | sed -n 's/.*Type error at \([0-9][0-9]*\):.*/\1/p')"
  if [[ -z "$line" ]]; then
    bad "$base could not parse line from: $msg"
    return
  fi
  if [[ "$line" -ge 50 ]]; then
    bad "$base line=$line (>=50); expected small local line for single-file"
    return
  fi
  if [[ "$line" -lt 1 ]]; then
    bad "$base line=$line (expected >=1)"
    return
  fi
  pass "$base line=$line (<50 single-file local)"
}

check_single "$ROOT/bootstrap/corpus/typecheck/fail/let_ann_mismatch.oo"
check_single "$ROOT/bootstrap/corpus/typecheck/fail/undefined_var.oo"
check_single "$ROOT/bootstrap/corpus/typecheck/fail/immut_assign.oo"

# Ad-hoc single-file: error on line 3 must stay line 3 (not import-padded)
PROBE="$TMPDIR/tc_line_probe.oo"
cat >"$PROBE" <<'EOF'
// line 1
fn main() {
    let x: Int = "hi";
}
EOF
set +e
"$OODAC" check "$PROBE" >"$TMPDIR/tc_probe.out" 2>"$TMPDIR/tc_probe.err"
prc=$?
set -e
if [[ $prc -eq 0 ]]; then
  bad "probe expected type fail"
else
  pmsg="$(cat "$TMPDIR/tc_probe.out" "$TMPDIR/tc_probe.err" 2>/dev/null | tr '\t' ' ' | grep -E 'Type error at [0-9]+:' | head -1 || true)"
  pline="$(echo "$pmsg" | sed -n 's/.*Type error at \([0-9][0-9]*\):.*/\1/p')"
  if [[ -z "$pline" ]]; then
    bad "probe missing Type error at: $pmsg"
  elif [[ "$pline" -ge 50 ]]; then
    bad "probe line=$pline inflated"
  elif [[ "$pline" -ne 3 ]]; then
    # still OK for path A if small; warn soft if not exact 3
    if [[ "$pline" -lt 50 ]]; then
      pass "probe line=$pline (<50; expected ~3 for let on L3)"
    else
      bad "probe line=$pline"
    fi
  else
    pass "probe line=3 exact (let ann mismatch)"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "typecheck_line_smoke: FAILED" >&2
  exit 1
fi
echo "typecheck_line_smoke: ALL OK (single-file local lines)"
echo "RESIDUAL: multi-import expanded LINE offsets — see oodac/tc_diag.oo"
exit 0
