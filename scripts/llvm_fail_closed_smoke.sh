#!/usr/bin/env bash
# job: M119 immune fail — missing LLVM tools + unsupported construct
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
LINK="$ROOT/scripts/llvm_link.sh"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
TMP="$TMPDIR/llvm_fail_closed_$$"
mkdir -p "$TMP/bin"
trap 'rm -rf "$TMP"' EXIT

fail=0
# 1) no clang/llc
for c in bash dirname mkdir cat rm timeout true false ls; do
  src=$(command -v "$c" 2>/dev/null) || continue
  ln -sf "$src" "$TMP/bin/$c"
done
echo 'define i32 @main() { ret i32 0 }' >"$TMP/t.ll"
set +e
env -i PATH="$TMP/bin" HOME="$HOME" TMPDIR="$TMP" /bin/bash "$LINK" -O0 "$TMP/t.ll" "$TMP/out" >"$TMP/no_llvm.out" 2>"$TMP/no_llvm.err"
ec=$?
set -e
if [[ $ec -eq 0 ]]; then
  echo "FAIL expected ERR_NO_LLVM non-zero" >&2
  fail=1
elif ! grep -q 'ERR_NO_LLVM' "$TMP/no_llvm.err" "$TMP/no_llvm.out" 2>/dev/null; then
  echo "FAIL missing ERR_NO_LLVM text" >&2
  cat "$TMP/no_llvm.err" >&2 || true
  fail=1
else
  echo "OK fail-closed no LLVM tools (exit=$ec)"
fi

# 2) unsupported for
cat >"$TMP/bad_for.oo" <<'EOF'
pub fn main() {
    for x in items {
        println(1);
    }
}
EOF
set +e
"$OODAC" emit-llvm "$TMP/bad_for.oo" >"$TMP/bad.ll" 2>"$TMP/bad.err"
ec2=$?
set -e
if [[ $ec2 -eq 0 ]]; then
  echo "FAIL expected non-zero on unsupported for" >&2
  fail=1
elif ! grep -qiE 'ERR|residual' "$TMP/bad.ll" "$TMP/bad.err" 2>/dev/null; then
  echo "FAIL missing residual ERR on bad for" >&2
  fail=1
else
  echo "OK fail-closed unsupported for (exit=$ec2)"
fi

# 3) M129 Secret sink refuse on emit-llvm (same dual-path as check)
SEC="$ROOT/fixtures/secret_sink_fail.oo"
if [[ -f "$SEC" ]]; then
  set +e
  "$OODAC" emit-llvm "$SEC" >"$TMP/sec.ll" 2>"$TMP/sec.err"
  ec3=$?
  set -e
  if [[ $ec3 -eq 0 ]]; then
    echo "FAIL expected non-zero emit-llvm on secret_sink_fail" >&2
    fail=1
  elif ! grep -qE $'ERR\tsecret|secret' "$TMP/sec.ll" "$TMP/sec.err" 2>/dev/null; then
    echo "FAIL missing secret ERR on llvm secret_sink_fail" >&2
    cat "$TMP/sec.err" >&2 || true
    fail=1
  else
    echo "OK fail-closed secret on emit-llvm (exit=$ec3)"
  fi
fi

if [[ "$fail" -ne 0 ]]; then
  echo "llvm_fail_closed_smoke: FAILED" >&2
  exit 1
fi
echo "llvm_fail_closed_smoke: PASSED"
exit 0
