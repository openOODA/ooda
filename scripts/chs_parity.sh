#!/usr/bin/env bash
# CHS parity driver (helpers: chs_parity_lib.sh)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=chs_parity_lib.sh
source "$SCRIPT_DIR/chs_parity_lib.sh"

echo "=== CHS token parity (pass) ==="
shopt -s nullglob
for f in "$ROOT"/bootstrap/corpus/lex/pass/*.oo \
         "$ROOT"/fixtures/hello.oo \
         "$ROOT"/fixtures/int_main.oo \
         "$ROOT"/fixtures/while_count.oo; do
  [[ -f "$f" ]] || continue
  compare_tokens "$f"
done

echo "=== CHS token fail-closed ==="
for f in "$ROOT"/bootstrap/corpus/lex/fail/*.oo; do
  [[ -f "$f" ]] || continue
  compare_tokens_fail "$f"
done

echo "=== CHS AST structure (real .oo parser, spans normalized) ==="
for f in "$ROOT"/bootstrap/corpus/parse/pass/*.oo \
         "$ROOT"/bootstrap/corpus/check/pass/ok_main.oo; do
  [[ -f "$f" ]] || continue
  compare_ast "$f"
done

echo "=== CHS check (real .oo cap/structure check) ==="
for f in "$ROOT"/bootstrap/corpus/check/pass/*.oo; do
  [[ -f "$f" ]] || continue
  compare_check "$f" pass
done
for f in "$ROOT"/bootstrap/corpus/check/fail/*.oo; do
  [[ -f "$f" ]] || continue
  compare_check "$f" fail
done

# R1 typecheck slice: oodac check must fail-closed on ann/return lit mismatches
# (parity with stage-0 `ooda check` exit code; oodac emits ERR\ttype\t…).
echo "=== R1 typecheck slice (oodac .oo vs stage-0 check) ==="
for f in "$ROOT"/bootstrap/corpus/typecheck/pass/*.oo; do
  [[ -f "$f" ]] || continue
  compare_check "$f" pass
done
for f in "$ROOT"/bootstrap/corpus/typecheck/fail/*.oo; do
  [[ -f "$f" ]] || continue
  set +e
  "$OODA" check "$f" >/dev/null 2>"$TMPDIR/tc0.err"
  ra=$?
  set -e
  b="$TMPDIR/tc_oodac.txt"
  rb=$(run_oodac_rc check "$f" "$b")
  if [[ $ra -eq 0 ]]; then
    echo "FAIL typecheck-fail: stage-0 accepted $f"
    fail=1
    continue
  fi
  if [[ $rb -eq 0 ]]; then
    echo "FAIL typecheck-fail: oodac accepted $f (must fail-closed)"
    cat "$b" | head -10
    fail=1
    continue
  fi
  if ! grep -qE $'^ERR\ttype\t|ERRtype' "$b" 2>/dev/null; then
    echo "FAIL typecheck-fail: oodac missing ERR type on $f"
    cat "$b" | head -10
    fail=1
    continue
  fi
  echo "OK typecheck fail-closed: $f (stage0_exit=$ra oodac_exit=$rb)"
done

echo "=== drift detector ==="
echo "KW_FAKE	1	1	x" >"$TMPDIR/drift_a.txt"
"$OODA" dump tokens "$ROOT/fixtures/int_main.oo" >"$TMPDIR/drift_b.txt"
if diff -q "$TMPDIR/drift_a.txt" "$TMPDIR/drift_b.txt" >/dev/null; then
  echo "FAIL drift detector"
  fail=1
else
  echo "OK drift detector"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "chs_parity: FAILED"
  exit 1
fi
echo "chs_parity: PASSED"
exit 0
