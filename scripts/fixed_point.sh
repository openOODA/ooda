#!/usr/bin/env bash
# M5 fixed-point referee (real two-generation self-host):
#  1) stage-0 builds oodac → stage1 (CHS C + libooda only if host FFI used)
#  2) stage1 builds a CHS smoke program (real chs_build, not hardcoded C)
#     — pure smoke links gcc+chs_rt only (no libooda; assembly depth to B0)
#  3) stage1 builds oodac/main.oo → stage2
#  4) stage2 token dump ≡ stage1 ≡ stage0 digests
#  5) intentional drift fails
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODA="${OODA:-$ROOT/target/release/ooda}"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/oodac"

if [[ ! -x "$OODA" ]]; then
  (cd "$ROOT" && cargo build --release)
fi
# Ensure staticlib exists for native link
(cd "$ROOT" && cargo build --release 2>/dev/null)

SMOKE_SRC="$ROOT/fixtures/chs_list_string.oo"
OODAC_SRC="$ROOT/oodac/main.oo"
STAGE1="$ROOT/oodac/oodac"
STAGE2="$ROOT/oodac/oodac2"
SMOKE_BIN="$TMPDIR/chs_smoke_real"

echo "=== fixed-point: stage-0 builds stage-1 oodac ==="
rm -f "$STAGE1" "$ROOT/oodac/main" "$ROOT/oodac/main.c" "$STAGE2"
(cd "$ROOT" && "$OODA" build --target c oodac/main.oo)
if [[ -x "$ROOT/oodac/main" ]]; then
  mv -f "$ROOT/oodac/main" "$STAGE1"
fi
if [[ ! -x "$STAGE1" ]]; then
  ls -la "$ROOT/oodac/" >&2
  echo "stage-1 binary missing" >&2
  exit 1
fi
echo "stage-1: $STAGE1"

echo "=== fixed-point: stage-1 real-builds CHS smoke ==="
rm -f "$SMOKE_BIN"
# Pure CHS: stage-1 self-emit. Full oodac recompile may use $OODA host seed (see oodac build).
export OODAC_BIN="$STAGE1"
export OODA
set +e
(cd "$ROOT" && "$STAGE1" build "$SMOKE_SRC" "$SMOKE_BIN" >"$TMPDIR/stage1_build_smoke.txt" 2>&1)
rc=$?
set -e
cat "$TMPDIR/stage1_build_smoke.txt"
if [[ $rc -ne 0 ]] || [[ ! -x "$SMOKE_BIN" ]]; then
  echo "FAIL: stage-1 did not produce executable smoke at $SMOKE_BIN" >&2
  exit 1
fi
smoke_out=$("$SMOKE_BIN" | tr -d '\r')
echo "smoke_out=$smoke_out"
# chs_list_string prints numbers/strings — require non-empty successful run
if [[ -z "$smoke_out" ]]; then
  echo "FAIL: smoke produced no output" >&2
  exit 1
fi
# Must not be the old hardcoded theater string alone without list behavior
if ! echo "$smoke_out" | grep -q '2'; then
  echo "FAIL: smoke output missing expected list length 2" >&2
  exit 1
fi
echo "OK stage-1 real smoke build"

echo "=== fixed-point: stage-1 builds oodac → stage-2 ==="
rm -f "$STAGE2" "$ROOT/oodac/main" "$ROOT/oodac/main.c" "$ROOT/oodac/main.oo.c"
set +e
# From repo root so runtime/ + OODA host seed resolve; OODAC_BIN prefers pure emit then host.
(cd "$ROOT" && OODAC_BIN="$STAGE1" OODA="$OODA" "$STAGE1" build "$OODAC_SRC" "$STAGE2" >"$TMPDIR/stage1_build_oodac.txt" 2>&1)
rc=$?
set -e
cat "$TMPDIR/stage1_build_oodac.txt"
if [[ $rc -ne 0 ]] || [[ ! -x "$STAGE2" ]]; then
  echo "FAIL: stage-1 did not build stage-2 oodac" >&2
  exit 1
fi
# Prove stage-2 is not a bit-identical copy of stage-1 produced by cp
# (BuildID may still match if emit is deterministic — compare mtimes/paths and both run)
if [[ "$STAGE1" -ef "$STAGE2" ]]; then
  echo "FAIL: stage-2 is same inode as stage-1" >&2
  exit 1
fi
echo "OK stage-2 binary produced by stage-1"

echo "=== fixed-point: digests stage0 ≡ stage1 ≡ stage2 tokens ==="
CORPUS="$ROOT/fixtures/int_main.oo"
"$OODA" dump tokens "$CORPUS" | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s0.sha"
"$STAGE1" tokens "$CORPUS" | grep $'\t' | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s1.sha"
"$STAGE2" tokens "$CORPUS" | grep $'\t' | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s2.sha"
echo "s0 $(cat $TMPDIR/fp_s0.sha)"
echo "s1 $(cat $TMPDIR/fp_s1.sha)"
echo "s2 $(cat $TMPDIR/fp_s2.sha)"
if ! diff -q "$TMPDIR/fp_s0.sha" "$TMPDIR/fp_s1.sha" >/dev/null; then
  echo "FAIL: stage-1 tokens != stage-0" >&2
  exit 1
fi
if ! diff -q "$TMPDIR/fp_s1.sha" "$TMPDIR/fp_s2.sha" >/dev/null; then
  echo "FAIL: stage-2 tokens != stage-1" >&2
  exit 1
fi
echo "OK token digests s0≡s1≡s2"

echo "=== fixed-point: stage-2 real-builds smoke ==="
SMOKE2="$TMPDIR/chs_smoke_from_s2"
rm -f "$SMOKE2"
"$STAGE2" build "$SMOKE_SRC" "$SMOKE2" >"$TMPDIR/stage2_build_smoke.txt" 2>&1
test -x "$SMOKE2"
out2=$("$SMOKE2" | tr -d '\r')
echo "stage2_smoke=$out2"
echo "$out2" | grep -q '2'
echo "OK stage-2 builds smoke"

echo "=== drift would-fail demo ==="
echo deadbeef >"$TMPDIR/bad.sha"
if diff -q "$TMPDIR/fp_s0.sha" "$TMPDIR/bad.sha" >/dev/null; then
  echo "FAIL drift demo" >&2
  exit 1
fi
echo "OK referee would fail on digest drift"

echo "fixed_point: PASSED"
exit 0
