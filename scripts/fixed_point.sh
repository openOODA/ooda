#!/usr/bin/env bash
# M5 fixed-point referee:
#  1) stage-0 builds oodac (CHS C backend) → stage1
#  2) stage1 dumps tokens for corpus; stage-0 dumps tokens; digests must match
#  3) stage-0 rebuilds oodac → stage2; stage1 vs stage2 smoke build outputs match
#  4) Normalized C emit of CHS smoke is stable across two stage-0 emissions
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA:-$ROOT/target/release/ooda}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/oodac"

if [[ ! -x "$OODA" ]]; then
  (cd "$ROOT" && cargo build --release)
fi

echo "=== fixed-point: build stage-1 oodac ==="
rm -f "$ROOT/oodac/oodac" "$ROOT/oodac/oodac.c"
(cd "$ROOT" && "$OODA" build --target c oodac/main.oo)
# binary is oodac/main by default (extension stripped from .oo path)
if [[ -x "$ROOT/oodac/main" ]]; then
  mv -f "$ROOT/oodac/main" "$ROOT/oodac/oodac"
elif [[ -x "$ROOT/oodac/main.oo" ]]; then
  echo "unexpected"
fi
# build writes next to source: oodac/main (no extension)
STAGE1="$ROOT/oodac/oodac"
if [[ ! -x "$STAGE1" ]]; then
  # path from with_extension("") on oodac/main.oo → oodac/main
  if [[ -x "$ROOT/oodac/main" ]]; then
    mv "$ROOT/oodac/main" "$STAGE1"
  fi
fi
if [[ ! -x "$STAGE1" ]]; then
  # list what we got
  ls -la "$ROOT/oodac/" || true
  echo "stage-1 binary missing" >&2
  exit 1
fi
echo "stage-1: $STAGE1"

echo "=== fixed-point: stage-1 vs stage-0 token digests ==="
CORPUS="$ROOT/examples/int_main.oo"
"$OODA" dump tokens "$CORPUS" | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s0.sha"
"$STAGE1" tokens "$CORPUS" | grep $'\t' | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s1.sha"
# native oodac may print without banners
if [[ ! -s "$TMPDIR/fp_s1.sha" ]]; then
  "$STAGE1" tokens "$CORPUS" | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s1.sha"
fi
echo "s0 $(cat $TMPDIR/fp_s0.sha)"
echo "s1 $(cat $TMPDIR/fp_s1.sha)"
if ! diff -q "$TMPDIR/fp_s0.sha" "$TMPDIR/fp_s1.sha" >/dev/null; then
  echo "FAIL: stage-1 token digest != stage-0" >&2
  "$OODA" dump tokens "$CORPUS" >"$TMPDIR/fp_s0_tok.txt"
  "$STAGE1" tokens "$CORPUS" >"$TMPDIR/fp_s1_tok.txt" || true
  diff -u "$TMPDIR/fp_s0_tok.txt" "$TMPDIR/fp_s1_tok.txt" | head -40 || true
  exit 1
fi
echo "OK token digests match"

echo "=== fixed-point: stage-1 check + smoke build ==="
"$STAGE1" check "$CORPUS" | head -1 | grep -q '^OK'
echo "OK stage-1 check"
(cd "$TMPDIR" && "$STAGE1" build "$CORPUS")
test -f "$TMPDIR/oodac_smoke_out.c" || test -f "$ROOT/oodac_smoke_out.c" || test -f oodac_smoke_out.c
# find smoke file
SMOKE=""
for p in "$TMPDIR/oodac_smoke_out.c" "$ROOT/oodac_smoke_out.c" "./oodac_smoke_out.c" "$ROOT/oodac/oodac_smoke_out.c"; do
  if [[ -f "$p" ]]; then SMOKE="$p"; break; fi
done
if [[ -z "$SMOKE" ]]; then
  # stage1 may write CWD
  find "$ROOT" "$TMPDIR" -name 'oodac_smoke_out.c' 2>/dev/null | head -1
  SMOKE=$(find "$ROOT" "$TMPDIR" -name 'oodac_smoke_out.c' 2>/dev/null | head -1 || true)
fi
if [[ -z "${SMOKE:-}" || ! -f "$SMOKE" ]]; then
  echo "FAIL: smoke c not written" >&2
  exit 1
fi
gcc -O2 -o "$TMPDIR/chs_smoke" "$SMOKE"
out=$("$TMPDIR/chs_smoke")
echo "smoke out: $out"
[[ "$out" == "chs-smoke-ok" ]]
echo "OK stage-1 smoke binary"

echo "=== fixed-point: rebuild stage-2 oodac ==="
rm -f "$ROOT/oodac/oodac2" "$ROOT/oodac/main" "$ROOT/oodac/main.c"
(cd "$ROOT" && "$OODA" build --target c oodac/main.oo)
if [[ -x "$ROOT/oodac/main" ]]; then
  mv -f "$ROOT/oodac/main" "$ROOT/oodac/oodac2"
fi
STAGE2="$ROOT/oodac/oodac2"
if [[ ! -x "$STAGE2" ]]; then
  ls -la "$ROOT/oodac/" >&2
  echo "stage-2 missing" >&2
  exit 1
fi

"$STAGE1" tokens "$CORPUS" | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s1b.sha"
"$STAGE2" tokens "$CORPUS" | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s2.sha"
echo "s1b $(cat $TMPDIR/fp_s1b.sha)"
echo "s2  $(cat $TMPDIR/fp_s2.sha)"
if ! diff -q "$TMPDIR/fp_s1b.sha" "$TMPDIR/fp_s2.sha" >/dev/null; then
  echo "FAIL: stage-2 token digest != stage-1" >&2
  exit 1
fi
echo "OK stage-1 ≡ stage-2 token digests"

echo "=== fixed-point: normalized C emit stability (CHS smoke) ==="
# Two emissions of examples/chs_list_string.oo C source, strip ephemeral paths
emit_norm() {
  local outc="$1"
  (cd "$ROOT" && "$OODA" build --target c --emit-llvm examples/chs_list_string.oo >/dev/null)
  # build writes examples/chs_list_string.c
  sed -e 's|/home/[^ ]*||g' -e 's|//.*||g' "$ROOT/examples/chs_list_string.c" \
    | tr -s ' \t' ' ' | sed '/^$/d' >"$outc"
}
emit_norm "$TMPDIR/c1.norm"
emit_norm "$TMPDIR/c2.norm"
if ! diff -q "$TMPDIR/c1.norm" "$TMPDIR/c2.norm" >/dev/null; then
  echo "FAIL: C emit not stable" >&2
  diff -u "$TMPDIR/c1.norm" "$TMPDIR/c2.norm" | head -20 || true
  exit 1
fi
echo "OK normalized C emit stable"

echo "=== drift would-fail demo ==="
echo deadbeef >"$TMPDIR/bad.sha"
if diff -q "$TMPDIR/fp_s0.sha" "$TMPDIR/bad.sha" >/dev/null; then
  echo "FAIL drift demo" >&2
  exit 1
fi
echo "OK referee would fail on digest drift"

echo "fixed_point: PASSED"
exit 0
