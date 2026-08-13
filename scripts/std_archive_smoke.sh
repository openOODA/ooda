#!/usr/bin/env bash
# job: pure std/archive magic-detect check + multi-build fixture
# in:  oodac, std/archive/{tar,zip,gzip}.oo, fixtures/std_archive_main.oo
# out: exit 0 if archive magic detect is real on pure path
# residual: full tar/zip/gzip decompress NOT product (detect only)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo ERR_NO_OODAC >&2; exit 1; }

for m in tar.oo zip.oo gzip.oo; do
  set +e
  "$OODAC" check "$ROOT/std/archive/$m" \
    >"$TMPDIR/std_arch_ck_$m.out" 2>"$TMPDIR/std_arch_ck_$m.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    bad "check std/archive/$m"
    head -8 "$TMPDIR/std_arch_ck_$m.err" 2>/dev/null || true
  else
    pass "check std/archive/$m"
  fi
done

# Residual honesty: no false decompress / DESIGN-done claims in product modules
for m in tar.oo zip.oo gzip.oo; do
  f="$ROOT/std/archive/$m"
  if grep -qiE 'stub_decompressed|full decompress|DESIGN.?done' "$f"; then
    bad "honesty std/archive/$m forbidden claim"
  else
    if grep -qiE 'decompress NOT product|full tar|full zip|full gzip' "$f"; then
      pass "honesty residual std/archive/$m"
    else
      bad "honesty residual missing std/archive/$m"
    fi
  fi
done

f="$ROOT/fixtures/std_archive_main.oo"
set +e
OODAC_BIN="$OODAC" "$OODAC" build "$f" "$TMPDIR/std_archive_main" \
  >"$TMPDIR/std_arch_b.out" 2>"$TMPDIR/std_arch_b.err"
rc=$?
set -e
if [[ $rc -ne 0 || ! -x "$TMPDIR/std_archive_main" ]]; then
  bad "build fixtures/std_archive_main.oo"
  head -12 "$TMPDIR/std_arch_b.err" 2>/dev/null || true
else
  set +e
  "$TMPDIR/std_archive_main" >"$TMPDIR/std_arch_r.out" 2>"$TMPDIR/std_arch_r.err"
  rr=$?
  set -e
  out="$(cat "$TMPDIR/std_arch_r.out" 2>/dev/null || true)"
  if [[ $rr -ne 0 ]]; then
    bad "run fixtures/std_archive_main.oo"
  elif ! echo "$out" | grep -q 'tar-detected'; then
    bad "run missing tar-detected (got: $out)"
  elif ! echo "$out" | grep -q 'zip-detected'; then
    bad "run missing zip-detected (got: $out)"
  elif ! echo "$out" | grep -q 'gzip-detected'; then
    bad "run missing gzip-detected (got: $out)"
  elif ! echo "$out" | grep -q 'not-tar'; then
    bad "run missing not-tar (got: $out)"
  else
    pass "build+run fixtures/std_archive_main.oo"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "std_archive_smoke: FAILED" >&2
  exit 1
fi
echo "std_archive_smoke: PASSED"
exit 0
