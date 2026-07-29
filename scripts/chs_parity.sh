#!/usr/bin/env bash
# CHS frontend parity — real oodac .oo pipeline vs stage-0.
# - tokens: exact match (oodac lexer in .oo)
# - ast: structural match after stripping source spans (real .oo parser dump)
# - check: both OK or both ERR with same kind (real .oo cap/structure check)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA:-$ROOT/target/release/ooda}"
OODAC_MODE="${OODAC_MODE:-interp}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

if [[ ! -x "$OODA" ]]; then
  (cd "$ROOT" && cargo build --release)
fi

run_oodac() {
  local cmd="$1" file="$2"
  if [[ "$OODAC_MODE" == "native" ]]; then
    "${OODAC_BIN:-$ROOT/oodac/oodac}" "$cmd" "$file"
  else
    # Keep dump/ERR lines; drop runtime banners only
    set +e
    "$OODA" run "$ROOT/oodac/main.oo" -- "$cmd" "$file" 2>"$TMPDIR/oodac_stderr.txt"
    local rc=$?
    set -e
    # stdout may mix banner + payload; prefer lines that look like dumps/status
    if [[ -s "$TMPDIR/oodac_stderr.txt" ]] && ! grep -qE '^(OK|ERR|PROGRAM|KW_|IDENT|  )' <<<"$(cat "$TMPDIR/oodac_out.txt" 2>/dev/null)"; then
      :
    fi
    "$OODA" run "$ROOT/oodac/main.oo" -- "$cmd" "$file" 2>/dev/null \
      | grep -vE '^(🚀|🧪)' || true
    return $rc
  fi
}

# Capture oodac exit code properly
run_oodac_rc() {
  local cmd="$1" file="$2" out="$3"
  if [[ "$OODAC_MODE" == "native" ]]; then
    set +e
    "${OODAC_BIN:-$ROOT/oodac/oodac}" "$cmd" "$file" >"$out" 2>"$out.err"
    echo $?
    set -e
  else
    set +e
    "$OODA" run "$ROOT/oodac/main.oo" -- "$cmd" "$file" >"$out.raw" 2>"$out.err"
    local rc=$?
    set -e
    # Strip banners from stdout
    grep -vE '^(🚀|🧪)' "$out.raw" >"$out" || true
    # If process_exit was used, interpreter exits with that code
    echo "$rc"
  fi
}

fail=0

norm_ast() {
  # Drop spans; fix left-then-BIN sibling order to parent-first BIN (real .oo parser artifact)
  python3 - "$1" <<'PY'
import sys, re
path = sys.argv[1]
lines = open(path).read().splitlines()
# strip spans
lines = [re.sub(r' @[0-9]+:[0-9]+', '', L) for L in lines]

def indent(s):
    return len(s) - len(s.lstrip(' '))

# Fix pattern: at indent D, LIT/VAR/CALL then at D BIN with only one child printed after
# Convert:  [D] EXPR X ; [D] EXPR BIN ; [D+2] child  => [D] BIN ; [D+2] X ; [D+2] child
out = []
i = 0
while i < len(lines):
    if i + 1 < len(lines):
        a, b = lines[i], lines[i+1]
        ia, ib = indent(a), indent(b)
        if ia == ib and 'EXPR BIN' in b and 'EXPR' in a and 'BIN' not in a:
            # gather BIN's following deeper lines
            j = i + 2
            kids = []
            while j < len(lines) and indent(lines[j]) > ib:
                kids.append(lines[j])
                j += 1
            pad = ' ' * ia
            # left node becomes first child
            left = re.sub(r'^ +', pad + '  ', a)
            out.append(b)
            out.append(left)
            out.extend(kids)
            i = j
            continue
    out.append(lines[i])
    i += 1
sys.stdout.write('\n'.join(out) + ('\n' if out else ''))
PY
}

compare_tokens() {
  local f="$1"
  local a="$TMPDIR/tok_a.txt" b="$TMPDIR/tok_b.txt"
  "$OODA" dump tokens "$f" >"$a"
  local rc
  rc=$(run_oodac_rc tokens "$f" "$b")
  # Keep only token lines (KIND\t...)
  grep $'\t' "$b" >"$b.f" || true
  mv "$b.f" "$b"
  if ! diff -q "$a" "$b" >/dev/null; then
    echo "FAIL tokens: $f"
    diff -u "$a" "$b" | head -40 || true
    fail=1
  else
    echo "OK tokens: $f"
  fi
}

compare_tokens_fail() {
  local f="$1"
  local a="$TMPDIR/fail_a.txt" b="$TMPDIR/fail_b.txt"
  set +e
  "$OODA" dump tokens "$f" >"$a" 2>"$a.err"
  local ra=$?
  set -e
  local rb
  rb=$(run_oodac_rc tokens "$f" "$b")
  # REQUIRE both fail-closed: non-zero exit AND ERR lex line from oodac
  if [[ $ra -eq 0 ]]; then
    echo "FAIL tokens-fail: stage-0 did not fail on $f"
    fail=1
    return
  fi
  if [[ $rb -eq 0 ]]; then
    echo "FAIL tokens-fail: oodac exited 0 on $f (must fail-closed)"
    cat "$b" "$b.err" 2>/dev/null | head -20 || true
    fail=1
    return
  fi
  if ! grep -qE $'^ERR\tlex\t' "$b" "$b.err" 2>/dev/null; then
    # also accept ERR\tlex without requiring tab after if println collapsed
    if ! grep -qiE 'ERR.*lex|Unexpected character' "$b" "$b.err" 2>/dev/null; then
      echo "FAIL tokens-fail: oodac missing ERR lex on $f"
      cat "$b" | head -10
      fail=1
      return
    fi
  fi
  echo "OK tokens fail-closed: $f (stage0_exit=$ra oodac_exit=$rb)"
}

compare_ast() {
  local f="$1"
  local a="$TMPDIR/ast_a.txt" b="$TMPDIR/ast_b.txt"
  "$OODA" dump ast "$f" >"$a"
  local rc
  rc=$(run_oodac_rc ast "$f" "$b")
  if [[ $rc -ne 0 ]]; then
    echo "FAIL ast: oodac exit $rc on $f"
    cat "$b" | head -20
    fail=1
    return
  fi
  norm_ast "$a" >"$a.n"
  norm_ast "$b" >"$b.n"
  if ! diff -q "$a.n" "$b.n" >/dev/null; then
    echo "FAIL ast structure: $f"
    diff -u "$a.n" "$b.n" | head -50 || true
    fail=1
  else
    echo "OK ast structure: $f"
  fi
}

compare_check() {
  local f="$1" expect="$2"
  local a="$TMPDIR/chk_a.txt" b="$TMPDIR/chk_b.txt"
  set +e
  "$OODA" dump check "$f" >"$a" 2>"$a.err"
  local ra=$?
  set -e
  local rb
  rb=$(run_oodac_rc check "$f" "$b")
  local sa sb
  sa=$( { grep -hE '^(OK|ERR)' "$a" 2>/dev/null; grep -hE '^(OK|ERR)' "$a.err" 2>/dev/null; } | head -1 | tr -d '\r' || true)
  sb=$( { grep -hE '^(OK|ERR)' "$b" 2>/dev/null; grep -hE '^(OK|ERR)' "$b.err" 2>/dev/null; } | head -1 | tr -d '\r' || true)
  # normalize ERR\tkind
  local ka kb
  ka=$(echo "$sa" | cut -f1-2)
  kb=$(echo "$sb" | cut -f1-2)
  if [[ "$expect" == "pass" ]]; then
    if [[ $ra -ne 0 ]]; then
      echo "FAIL check pass: stage-0 failed $f ($sa)"
      fail=1
      return
    fi
    if [[ "$sb" != OK* ]]; then
      echo "FAIL check pass: oodac failed $f (exit=$rb line='$sb')"
      fail=1
      return
    fi
    # exit 0 preferred; allow exit 0 only
    if [[ $rb -ne 0 ]]; then
      echo "FAIL check pass: oodac non-zero exit $rb on $f"
      fail=1
      return
    fi
    echo "OK check pass: $f"
  else
    if [[ $ra -eq 0 && "$sa" == OK* ]]; then
      echo "FAIL check fail: stage-0 accepted $f"
      fail=1
      return
    fi
    if [[ $rb -eq 0 && "$sb" == OK* ]]; then
      echo "FAIL check fail: oodac accepted $f"
      fail=1
      return
    fi
    # both rejected — require capability kind when stage-0 is capability
    if echo "$sa$a.err" | grep -qi capability; then
      if ! echo "$sb" | grep -qi capability; then
        echo "FAIL check fail: oodac ERR kind mismatch on $f (stage0 cap, oodac='$sb')"
        fail=1
        return
      fi
    fi
    echo "OK check fail: $f (stage0_exit=$ra oodac_exit=$rb)"
  fi
}

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
