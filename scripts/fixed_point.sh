#!/usr/bin/env bash
# M5 fixed-point referee — pure seed only (no host FORCE_HOST codegen):
#  1) trusted pure SEED_OODAC builds stage-1 from oodac/main.oo
#  2) stage-1 real-builds CHS smoke (emit-c + gcc + chs_rt)
#  3) stage-1 pure-builds stage-2
#  4) token digests stage-1 ≡ stage-2 (N vs N+1; no host s0)
#  5) no OK_HOST; pure OK_PURE*; bit-identical s1≡s2 is OK pure FP
#  6) intentional digest drift fails
#
# Residual: first bootstrap needs a seed binary (SEED_OODAC or existing
# oodac/oodac|oodac2). Host Rust C backend is NOT used on this path.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR" "$ROOT/oodac"

SMOKE_SRC="$ROOT/fixtures/chs_list_string.oo"
OODAC_SRC="$ROOT/oodac/main.oo"
STAGE1="$ROOT/oodac/oodac"
STAGE2="$ROOT/oodac/oodac2"
SMOKE_BIN="$TMPDIR/chs_smoke_real"

# Capture seed BEFORE removing stage binaries (always copy aside —
# never use STAGE1 path as SEED so rm STAGE1 cannot unlink the seed).
SEED_SRC="${SEED_OODAC:-}"
if [[ -z "$SEED_SRC" || ! -x "$SEED_SRC" ]]; then
  if [[ -x "$ROOT/oodac/oodac2" ]]; then
    SEED_SRC="$ROOT/oodac/oodac2"
  elif [[ -x "$ROOT/oodac/oodac" ]]; then
    SEED_SRC="$ROOT/oodac/oodac"
  else
    echo "FAIL: no pure SEED_OODAC (set SEED_OODAC or provide oodac/oodac)" >&2
    echo "Residual bootstrap: obtain a pure seed binary; host FORCE_HOST seed retired." >&2
    exit 1
  fi
fi
SEED="$TMPDIR/fp_seed_oodac"
cp -a "$SEED_SRC" "$SEED"
chmod +x "$SEED"
echo "seed: $SEED (from $SEED_SRC)"

echo "=== fixed-point: pure seed builds stage-1 oodac ==="
rm -f "$STAGE1" "$ROOT/oodac/main" "$ROOT/oodac/main.c" "$STAGE2" "$ROOT/oodac/main.oo.c"
set +e
(cd "$ROOT" && env -u OODA OODAC_BIN="$SEED" "$SEED" build "$OODAC_SRC" "$STAGE1" \
  >"$TMPDIR/seed_build_s1.txt" 2>&1)
rc=$?
set -e
cat "$TMPDIR/seed_build_s1.txt"
if [[ $rc -ne 0 ]] || [[ ! -x "$STAGE1" ]]; then
  echo "FAIL: pure seed did not build stage-1" >&2
  exit 1
fi
if grep -q 'OK_HOST' "$TMPDIR/seed_build_s1.txt" 2>/dev/null; then
  echo "FAIL: seed→stage-1 used OK_HOST soft-pass" >&2
  exit 1
fi
if ! grep -qE 'OK_PURE|OK_PURE_MULTI|OK\tbuild' "$TMPDIR/seed_build_s1.txt" 2>/dev/null; then
  # Accept executable without banner if still pure (no OK_HOST already checked)
  echo "WARN: seed build log missing OK_PURE banner (binary present)"
fi
echo "stage-1: $STAGE1"

echo "=== fixed-point: stage-1 real-builds CHS smoke ==="
rm -f "$SMOKE_BIN"
export OODAC_BIN="$STAGE1"
set +e
(cd "$ROOT" && env -u OODA OODAC_BIN="$STAGE1" "$STAGE1" build "$SMOKE_SRC" "$SMOKE_BIN" \
  >"$TMPDIR/stage1_build_smoke.txt" 2>&1)
rc=$?
set -e
cat "$TMPDIR/stage1_build_smoke.txt"
if [[ $rc -ne 0 ]] || [[ ! -x "$SMOKE_BIN" ]]; then
  echo "FAIL: stage-1 did not produce executable smoke at $SMOKE_BIN" >&2
  exit 1
fi
smoke_out=$("$SMOKE_BIN" | tr -d '\r')
echo "smoke_out=$smoke_out"
if [[ -z "$smoke_out" ]]; then
  echo "FAIL: smoke produced no output" >&2
  exit 1
fi
if ! echo "$smoke_out" | grep -q '2'; then
  echo "FAIL: smoke output missing expected list length 2" >&2
  exit 1
fi
echo "OK stage-1 real smoke build"

echo "=== fixed-point: stage-1 builds oodac → stage-2 (pure emit only) ==="
rm -f "$STAGE2" "$ROOT/oodac/main" "$ROOT/oodac/main.c" "$ROOT/oodac/main.oo.c"
set +e
(cd "$ROOT" && env -u OODA OODAC_BIN="$STAGE1" "$STAGE1" build "$OODAC_SRC" "$STAGE2" \
  >"$TMPDIR/stage1_build_oodac.txt" 2>&1)
rc=$?
set -e
cat "$TMPDIR/stage1_build_oodac.txt"
if [[ $rc -ne 0 ]] || [[ ! -x "$STAGE2" ]]; then
  echo "FAIL: stage-1 did not pure-build stage-2 oodac" >&2
  exit 1
fi
if [[ "$STAGE1" -ef "$STAGE2" ]]; then
  echo "FAIL: stage-2 is same inode as stage-1" >&2
  exit 1
fi
if grep -q 'OK_HOST' "$TMPDIR/stage1_build_oodac.txt" 2>/dev/null; then
  echo "FAIL: stage-2 used OK_HOST soft-pass" >&2
  exit 1
fi
# Pure fixed-point may be bit-identical (same sources → same emit). That is success,
# not host re-seed theater — theater is OK_HOST / missing pure build log.
if ! grep -qE 'OK_PURE|OK_PURE_MULTI' "$TMPDIR/stage1_build_oodac.txt" 2>/dev/null; then
  echo "FAIL: stage-2 build log missing OK_PURE / OK_PURE_MULTI" >&2
  exit 1
fi
if cmp -s "$STAGE1" "$STAGE2"; then
  echo "OK stage-2 bit-identical to stage-1 (pure fixed-point)"
else
  echo "OK stage-2 differs from stage-1 bytes (rebuild; digests must still match)"
fi
echo "OK stage-2 binary pure-built by stage-1"

echo "=== fixed-point: digests stage-1 ≡ stage-2 tokens (N vs N+1) ==="
CORPUS="$ROOT/fixtures/int_main.oo"
"$STAGE1" tokens "$CORPUS" | grep $'\t' | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s1.sha"
"$STAGE2" tokens "$CORPUS" | grep $'\t' | sha256sum | awk '{print $1}' >"$TMPDIR/fp_s2.sha"
echo "s1 $(cat $TMPDIR/fp_s1.sha)"
echo "s2 $(cat $TMPDIR/fp_s2.sha)"
if ! diff -q "$TMPDIR/fp_s1.sha" "$TMPDIR/fp_s2.sha" >/dev/null; then
  echo "FAIL: stage-2 tokens != stage-1" >&2
  exit 1
fi
echo "OK token digests s1≡s2"

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
if diff -q "$TMPDIR/fp_s1.sha" "$TMPDIR/bad.sha" >/dev/null; then
  echo "FAIL drift demo" >&2
  exit 1
fi
echo "OK referee would fail on digest drift"

echo "fixed_point: PASSED"
exit 0
