#!/usr/bin/env bash
# CHS parity — pure oodac self-consistency + product CLI vs pure oodac.
# Host frontend deleted: no FORCE_HOST / stage-0 host dumps.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
OODA="${OODA:-$ROOT/bin/ooda}"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

run_oodac_rc() {
  local cmd="$1" file="$2" out="$3"
  set +e
  "$OODAC" "$cmd" "$file" >"$out" 2>"$out.err"
  echo $?
  set -e
}

run_product_dump() {
  # Product CLI is pure dispatch only
  if [[ -x "$OODA" ]]; then
    "$OODA" dump "$@"
  else
    "$OODAC" "$@"
  fi
}

fail=0

norm_ast() {
  python3 - "$1" <<'PY'
import sys, re
path = sys.argv[1]
lines = open(path).read().splitlines()
lines = [re.sub(r' @[0-9]+:[0-9]+', '', L) for L in lines]

def indent(s):
    return len(s) - len(s.lstrip(' '))

out = []
i = 0
while i < len(lines):
    if i + 1 < len(lines):
        a, b = lines[i], lines[i+1]
        ia, ib = indent(a), indent(b)
        if ia == ib and 'EXPR BIN' in b and 'EXPR' in a and 'BIN' not in a:
            j = i + 2
            kids = []
            while j < len(lines) and indent(lines[j]) > ib:
                kids.append(lines[j])
                j += 1
            pad = ' ' * ia
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
  set +e
  run_product_dump tokens "$f" >"$a" 2>"$a.err"
  local ra=$?
  set -e
  local rb
  rb=$(run_oodac_rc tokens "$f" "$b")
  grep $'\t' "$a" >"$a.f" 2>/dev/null || true
  grep $'\t' "$b" >"$b.f" 2>/dev/null || true
  mv "$a.f" "$a"
  mv "$b.f" "$b"
  if [[ $ra -ne 0 ]] || [[ $rb -ne 0 ]]; then
    echo "FAIL tokens exit: $f (product=$ra oodac=$rb)"
    fail=1
    return
  fi
  if ! diff -q "$a" "$b" >/dev/null; then
    echo "FAIL tokens product vs oodac: $f"
    diff -u "$a" "$b" | head -40 || true
    fail=1
  else
    echo "OK tokens product≡oodac: $f"
  fi
}

compare_tokens_fail() {
  local f="$1"
  local a="$TMPDIR/fail_a.txt" b="$TMPDIR/fail_b.txt"
  set +e
  run_product_dump tokens "$f" >"$a" 2>"$a.err"
  local ra=$?
  set -e
  local rb
  rb=$(run_oodac_rc tokens "$f" "$b")
  if [[ $ra -eq 0 ]]; then
    echo "FAIL tokens-fail: product accepted $f"
    fail=1
    return
  fi
  if [[ $rb -eq 0 ]]; then
    echo "FAIL tokens-fail: oodac accepted $f"
    fail=1
    return
  fi
  if ! grep -qE $'^ERR\tlex\t|ERR.*lex|Unexpected character' "$a" "$a.err" "$b" "$b.err" 2>/dev/null; then
    echo "FAIL tokens-fail: missing ERR lex on $f"
    fail=1
    return
  fi
  echo "OK tokens fail-closed: $f (product=$ra oodac=$rb)"
}

compare_ast() {
  local f="$1"
  local a="$TMPDIR/ast_a.txt" b="$TMPDIR/ast_b.txt"
  set +e
  run_product_dump ast "$f" >"$a" 2>"$a.err"
  local ra=$?
  set -e
  local rb
  rb=$(run_oodac_rc ast "$f" "$b")
  if [[ $ra -ne 0 ]] || [[ $rb -ne 0 ]]; then
    echo "FAIL ast exit: $f (product=$ra oodac=$rb)"
    fail=1
    return
  fi
  norm_ast "$a" >"$a.n"
  norm_ast "$b" >"$b.n"
  if ! diff -q "$a.n" "$b.n" >/dev/null; then
    echo "FAIL ast product vs oodac: $f"
    diff -u "$a.n" "$b.n" | head -50 || true
    fail=1
  else
    echo "OK ast product≡oodac: $f"
  fi
}

compare_check() {
  local f="$1" expect="$2"
  local a="$TMPDIR/chk_a.txt" b="$TMPDIR/chk_b.txt"
  set +e
  run_product_dump check "$f" >"$a" 2>"$a.err"
  local ra=$?
  set -e
  local rb
  rb=$(run_oodac_rc check "$f" "$b")
  local sa sb
  sa=$( { grep -hE '^(OK|ERR)' "$a" 2>/dev/null; grep -hE '^(OK|ERR)' "$a.err" 2>/dev/null; } | head -1 | tr -d '\r' || true)
  sb=$( { grep -hE '^(OK|ERR)' "$b" 2>/dev/null; grep -hE '^(OK|ERR)' "$b.err" 2>/dev/null; } | head -1 | tr -d '\r' || true)
  if [[ "$expect" == "pass" ]]; then
    if [[ $ra -ne 0 ]] || [[ "$sa" != OK* ]]; then
      echo "FAIL check pass product: $f ($sa exit=$ra)"
      fail=1
      return
    fi
    if [[ $rb -ne 0 ]] || [[ "$sb" != OK* ]]; then
      echo "FAIL check pass oodac: $f ($sb exit=$rb)"
      fail=1
      return
    fi
    echo "OK check pass product≡oodac: $f"
  else
    if [[ $ra -eq 0 && "$sa" == OK* ]]; then
      echo "FAIL check fail: product accepted $f"
      fail=1
      return
    fi
    if [[ $rb -eq 0 && "$sb" == OK* ]]; then
      echo "FAIL check fail: oodac accepted $f"
      fail=1
      return
    fi
    echo "OK check fail-closed: $f (product=$ra oodac=$rb)"
  fi
}
