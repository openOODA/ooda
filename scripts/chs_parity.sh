#!/usr/bin/env bash
# CHS frontend parity: stage-0 dumps vs oodac (interpreter or OODAC_BIN).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA:-$ROOT/target/release/ooda}"
OODAC_MODE="${OODAC_MODE:-interp}" # interp | native
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODA" ]]; then
  (cd "$ROOT" && cargo build --release)
fi

run_oodac() {
  local cmd="$1" file="$2"
  if [[ "$OODAC_MODE" == "native" ]]; then
    local bin="${OODAC_BIN:-$ROOT/oodac/oodac}"
    "$bin" "$cmd" "$file"
  else
    # Strip runtime banners; keep dump lines (contain TAB) or OK/ERR
    "$OODA" run "$ROOT/oodac/main.oo" -- "$cmd" "$file" 2>/dev/null \
      | grep -E $'\t|^OK|^ERR|^PROGRAM|^  ITEM' || true
  fi
}

fail=0
compare_tokens() {
  local f="$1"
  local a="$TMPDIR/parity_a.txt" b="$TMPDIR/parity_b.txt"
  "$OODA" dump tokens "$f" >"$a"
  run_oodac tokens "$f" >"$b"
  if ! diff -q "$a" "$b" >/dev/null; then
    echo "FAIL tokens: $f"
    diff -u "$a" "$b" | head -40 || true
    fail=1
  else
    echo "OK tokens: $f"
  fi
}

compare_check() {
  local f="$1" expect="$2" # pass|fail
  local a="$TMPDIR/chk_a.txt" b="$TMPDIR/chk_b.txt"
  set +e
  "$OODA" dump check "$f" >"$a" 2>/dev/null
  local ra=$?
  set -e
  run_oodac check "$f" >"$b"
  local sa sb
  sa=$(head -1 "$a" || true)
  sb=$(head -1 "$b" || true)
  if [[ "$expect" == "pass" ]]; then
    if [[ "$sa" == OK* && "$sb" == OK* ]]; then
      echo "OK check pass: $f"
    else
      echo "FAIL check pass: $f (stage0='$sa' oodac='$sb')"
      fail=1
    fi
  else
    # both should not be pure OK
    if [[ "$sa" == OK* ]]; then
      # stage-0 might print human error and exit 1 with ERR line on stderr path
      if [[ $ra -eq 0 ]]; then
        echo "FAIL check fail corpus passed stage0: $f"
        fail=1
        return
      fi
    fi
    if [[ "$sb" == OK* ]]; then
      echo "FAIL check fail corpus passed oodac: $f"
      fail=1
    else
      echo "OK check fail: $f (stage0_exit=$ra oodac='$sb')"
    fi
  fi
}

echo "=== CHS token parity (pass corpus) ==="
shopt -s nullglob
for f in "$ROOT"/bootstrap/corpus/lex/pass/*.oo "$ROOT"/examples/hello.oo "$ROOT"/examples/int_main.oo "$ROOT"/examples/while_count.oo; do
  [[ -f "$f" ]] || continue
  compare_tokens "$f"
done

echo "=== CHS check corpus ==="
for f in "$ROOT"/bootstrap/corpus/check/pass/*.oo; do
  [[ -f "$f" ]] || continue
  compare_check "$f" pass
done
for f in "$ROOT"/bootstrap/corpus/check/fail/*.oo; do
  [[ -f "$f" ]] || continue
  compare_check "$f" fail
done

echo "=== AST smoke (stage-0 dump exists; oodac emits PROGRAM) ==="
for f in "$ROOT"/bootstrap/corpus/parse/pass/*.oo "$ROOT"/examples/int_main.oo; do
  [[ -f "$f" ]] || continue
  "$OODA" dump ast "$f" >"$TMPDIR/ast_a.txt"
  run_oodac ast "$f" >"$TMPDIR/ast_b.txt"
  if grep -q '^PROGRAM' "$TMPDIR/ast_a.txt" && grep -q '^PROGRAM' "$TMPDIR/ast_b.txt"; then
    echo "OK ast PROGRAM: $f"
  else
    echo "FAIL ast: $f"
    fail=1
  fi
done

# Prove non-zero on intentional token drift
echo "=== drift detector ==="
echo "KW_FAKE	1	1	x" >"$TMPDIR/drift_a.txt"
"$OODA" dump tokens "$ROOT/examples/int_main.oo" >"$TMPDIR/drift_b.txt"
if diff -q "$TMPDIR/drift_a.txt" "$TMPDIR/drift_b.txt" >/dev/null; then
  echo "FAIL: drift detector did not differ"
  fail=1
else
  echo "OK drift detector differs (would fail CI)"
fi

if [[ "$fail" -ne 0 ]]; then
  echo "chs_parity: FAILED"
  exit 1
fi
echo "chs_parity: PASSED"
exit 0
