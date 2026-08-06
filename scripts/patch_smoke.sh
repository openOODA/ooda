#!/usr/bin/env bash
# job: ooda patch rails (pass + fail-closed + path safety)
# in:  bin/ooda + scripts/ooda_patch.sh + oodac for --check
# out: exit 0 if all patch rails green
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA:-$ROOT/bin/ooda}"
export OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: need $OODA" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

WORK="$TMPDIR/patch_smoke_$$"
mkdir -p "$WORK"
cp "$ROOT/fixtures/patch_add.oo" "$WORK/target.oo"
cp "$ROOT/fixtures/patch_add_body.txt" "$WORK/body.txt"

# --- pass: replace_fn via CLI --with ---
set +e
"$OODA" patch "$WORK/target.oo" --replace-fn add --with "$WORK/body.txt" \
  >"$TMPDIR/patch_ok.out" 2>"$TMPDIR/patch_ok.err"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  bad "patch replace_fn exit=$rc"
  cat "$TMPDIR/patch_ok.err" | head -20 || true
elif ! grep -q 'return a - b' "$WORK/target.oo"; then
  bad "patch body not applied"
else
  pass "patch replace_fn applied"
fi

# --- pass: --check after patch ---
cp "$ROOT/fixtures/patch_add.oo" "$WORK/target2.oo"
# body that still typechecks (subtract)
set +e
"$OODA" patch "$WORK/target2.oo" --replace-fn add --with "$WORK/body.txt" --check \
  >"$TMPDIR/patch_chk.out" 2>"$TMPDIR/patch_chk.err"
rc=$?
set -e
if [[ $rc -ne 0 ]]; then
  bad "patch --check exit=$rc"
else
  pass "patch --check"
fi

# --- fail: missing function ---
cp "$ROOT/fixtures/patch_add.oo" "$WORK/miss.oo"
set +e
"$OODA" patch "$WORK/miss.oo" --replace-fn nosuch --with "$WORK/body.txt" \
  >"$TMPDIR/patch_miss.out" 2>"$TMPDIR/patch_miss.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "missing fn should fail"
else
  pass "missing fn fail-closed"
fi

# --- fail: path traversal ---
set +e
"$OODA" patch "../etc/passwd" --replace-fn add --with "$WORK/body.txt" \
  >"$TMPDIR/patch_trav.out" 2>"$TMPDIR/patch_trav.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "path traversal should fail"
else
  pass "path traversal rejected"
fi

# --- pass: JSON stdin ---
cp "$ROOT/fixtures/patch_add.oo" "$WORK/json.oo"
set +e
printf '%s' '{"op":"replace_fn","name":"add","body":"return a * b;"}' \
  | "$OODA" patch "$WORK/json.oo" \
  >"$TMPDIR/patch_json.out" 2>"$TMPDIR/patch_json.err"
rc=$?
set -e
if [[ $rc -ne 0 ]] || ! grep -q 'return a \* b' "$WORK/json.oo"; then
  bad "JSON stdin patch exit=$rc"
  cat "$TMPDIR/patch_json.err" | head -10 || true
else
  pass "JSON stdin replace_fn"
fi

# --- fail: unknown JSON op ---
cp "$ROOT/fixtures/patch_add.oo" "$WORK/badop.oo"
set +e
printf '%s' '{"op":"eval","name":"add","body":"x"}' \
  | "$OODA" patch "$WORK/badop.oo" \
  >"$TMPDIR/patch_badop.out" 2>"$TMPDIR/patch_badop.err"
rc=$?
set -e
if [[ $rc -eq 0 ]]; then
  bad "unknown op should fail"
else
  pass "unknown op fail-closed"
fi

# --- fail: free-form shell payload not executed (body is text only) ---
cp "$ROOT/fixtures/patch_add.oo" "$WORK/shell.oo"
set +e
printf '%s' '{"op":"replace_fn","name":"add","body":"return 1; // $(touch /tmp/ooda_patch_pwned)"}' \
  | "$OODA" patch "$WORK/shell.oo" \
  >"$TMPDIR/patch_shell.out" 2>"$TMPDIR/patch_shell.err"
rc=$?
set -e
if [[ -e /tmp/ooda_patch_pwned ]]; then
  bad "shell payload was executed"
  rm -f /tmp/ooda_patch_pwned
elif [[ $rc -ne 0 ]]; then
  bad "text body with shell-looking chars should still apply as text"
else
  pass "body never shell-evaled"
fi

rm -rf "$WORK"

if [[ $fail -ne 0 ]]; then
  echo "patch_smoke: FAILED" >&2
  exit 1
fi
echo "patch_smoke: PASSED"
exit 0
