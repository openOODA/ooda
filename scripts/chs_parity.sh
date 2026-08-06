#!/usr/bin/env bash
# CHS parity driver — product pure CLI vs pure oodac (host frontend deleted)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=chs_parity_lib.sh
source "$SCRIPT_DIR/chs_parity_lib.sh"

echo "=== CHS token parity product≡oodac (pass) ==="
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

echo "=== CHS AST structure product≡oodac ==="
for f in "$ROOT"/bootstrap/corpus/parse/pass/*.oo \
         "$ROOT"/bootstrap/corpus/check/pass/ok_main.oo; do
  [[ -f "$f" ]] || continue
  compare_ast "$f"
done

echo "=== CHS check product≡oodac ==="
for f in "$ROOT"/bootstrap/corpus/check/pass/*.oo; do
  [[ -f "$f" ]] || continue
  compare_check "$f" pass
done
for f in "$ROOT"/bootstrap/corpus/check/fail/*.oo; do
  [[ -f "$f" ]] || continue
  compare_check "$f" fail
done

# Typecheck slice: pure oodac must fail-closed (product dump check = oodac)
echo "=== R1 typecheck fail-closed (pure oodac) ==="
for f in "$ROOT"/bootstrap/corpus/typecheck/fail/*.oo; do
  [[ -f "$f" ]] || continue
  b="$TMPDIR/tc_oodac.txt"
  rb=$(run_oodac_rc check "$f" "$b")
  if [[ $rb -eq 0 ]]; then
    echo "FAIL typecheck-fail: oodac accepted $f"
    fail=1
    continue
  fi
  if ! grep -qiE 'ERR' "$b" "$b.err" 2>/dev/null; then
    echo "FAIL typecheck-fail: oodac missing ERR on $f"
    fail=1
    continue
  fi
  echo "OK typecheck fail-closed: $f (oodac_exit=$rb)"
done

echo "=== drift detector ==="
echo "KW_FAKE	1	1	x" >"$TMPDIR/drift_a.txt"
"$OODAC" tokens "$ROOT/fixtures/int_main.oo" >"$TMPDIR/drift_b.txt" 2>/dev/null || true
if diff -q "$TMPDIR/drift_a.txt" "$TMPDIR/drift_b.txt" >/dev/null; then
  echo "FAIL drift detector"
  fail=1
else
  echo "OK drift detector"
fi

# Anti: host modules must stay gone
if [[ -d "$ROOT/src/lexer" ]] || [[ -d "$ROOT/src/typecheck" ]] || [[ -d "$ROOT/src/codegen_c" ]]; then
  echo "FAIL host spine modules reappeared"
  fail=1
else
  echo "OK host spine modules absent"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "chs_parity: FAILED"
  exit 1
fi
echo "chs_parity: PASSED"
exit 0
