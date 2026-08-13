#!/usr/bin/env bash
# Merkle persist/load of a byte blob. Language holo_persist stays refused.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCR="${OODA_HOLO_SCRATCH:-${TMPDIR:-/tmp}/ooda-holo-smoke}"
mkdir -p "$SCR"
cat >"$SCR/holo_harness.c" <<'C'
#include "oo_holo.h"
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
  const unsigned char msg[] = "openOODA merkle blob";
  unsigned char root1[32], root2[32], buf[64];
  size_t n = 0;
  int i;
  const char *path;
  if (argc < 2) {
    fprintf(stderr, "usage: holo_harness PATH\n");
    return 1;
  }
  path = argv[1];
  if (oo_holo_path_ok("../evil") || oo_holo_path_ok("") || oo_holo_path_ok(0)) {
    fprintf(stderr, "path gate failed\n");
    return 1;
  }
  if (oo_holo_persist(path, msg, sizeof msg) != 0) {
    fprintf(stderr, "persist fail\n");
    return 1;
  }
  if (oo_holo_load(path, buf, sizeof buf, &n, root1) != 0) {
    fprintf(stderr, "load fail\n");
    return 1;
  }
  if (n != sizeof msg || memcmp(buf, msg, n) != 0) {
    fprintf(stderr, "payload mismatch\n");
    return 1;
  }
  if (oo_holo_root(msg, sizeof msg, root2) != 0 || memcmp(root1, root2, 32) != 0) {
    fprintf(stderr, "root mismatch\n");
    return 1;
  }
  printf("holo_roundtrip n=%zu root=", n);
  for (i = 0; i < 8; i++) {
    printf("%02x", root1[i]);
  }
  printf("\n");
  return 0;
}
C
gcc -O0 -I"$ROOT/runtime" "$SCR/holo_harness.c" "$ROOT/runtime/oo_holo.c" \
  "$ROOT/runtime/oo_sha256.c" -o "$SCR/holo_harness"
cat >"$SCR/sha_kat.c" <<'C'
#include "oo_sha256.h"
#include <stdio.h>
#include <string.h>
int main(void) {
  unsigned char out[32];
  const char *want = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
  char hex[65];
  int i;
  oo_sha256((const unsigned char *)"", 0, out);
  for (i = 0; i < 32; i++) {
    sprintf(hex + i * 2, "%02x", out[i]);
  }
  hex[64] = 0;
  if (strcmp(hex, want) != 0) {
    fprintf(stderr, "sha256 empty KAT fail %s\n", hex);
    return 1;
  }
  printf("sha256 empty KAT OK\n");
  return 0;
}
C
gcc -O0 -I"$ROOT/runtime" "$SCR/sha_kat.c" "$ROOT/runtime/oo_sha256.c" -o "$SCR/sha_kat"
"$SCR/sha_kat"
"$SCR/holo_harness" "$SCR/round.bin"
# Flip last byte of the store. Load must fail.
cp "$SCR/round.bin" "$SCR/round.bin.bad"
sz=$(wc -c < "$SCR/round.bin.bad")
printf '\001' | dd of="$SCR/round.bin.bad" bs=1 seek=$((sz - 1)) conv=notrunc status=none
cat >"$SCR/holo_tamper.c" <<'C'
#include "oo_holo.h"
#include <stdio.h>
int main(int argc, char **argv) {
  unsigned char buf[64], root[32];
  size_t n = 0;
  if (argc < 2) return 2;
  if (oo_holo_load(argv[1], buf, sizeof buf, &n, root) == 0) {
    fprintf(stderr, "tamper load must fail\n");
    return 1;
  }
  return 0;
}
C
gcc -O0 -I"$ROOT/runtime" "$SCR/holo_tamper.c" "$ROOT/runtime/oo_holo.c" \
  "$ROOT/runtime/oo_sha256.c" -o "$SCR/holo_tamper"
"$SCR/holo_tamper" "$SCR/round.bin.bad"

OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
if [[ -x "$OODAC" && -f "$ROOT/fixtures/residual_holo_fail.oo" ]]; then
  set +e
  out=$("$OODAC" check "$ROOT/fixtures/residual_holo_fail.oo" 2>&1)
  rc=$?
  set -e
  if [[ $rc -ne 0 ]] && echo "$out" | grep -qiE 'HOLOGRAPHIC|residual'; then
    echo "OK language holo_persist still refused"
  else
    echo "FAIL language refuse expected (rc=$rc out=$out)" >&2
    exit 1
  fi
else
  echo "OK skip live oodac holo refuse (no binary or fixture)"
fi
echo "oo_holo_smoke: PASSED"
