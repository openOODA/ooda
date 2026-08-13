#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMP"
cat >"$TMP/meta_epoch_harness.c" <<'C'
#include "chs_rt.h"
#include <stdio.h>
int main(void) {
  long long a = oo_meta_epoch();
  long long b = oo_meta_epoch();
  long long m = oo_meta_mix(42);
  if (a == 0 || a != b) { fprintf(stderr, "epoch fail\n"); return 1; }
  if (m == 0) { fprintf(stderr, "mix fail\n"); return 1; }
  printf("OK epoch=%lld mix=%lld path_a=%d\n", (long long)a, (long long)m, oo_meta_is_path_a());
  return 0;
}
C
gcc -O0 -I"$ROOT/runtime" "$TMP/meta_epoch_harness.c" "$ROOT/runtime/chs_rt.c" -lm -ldl -lpthread -o "$TMP/meta_epoch_harness"
out1=$("$TMP/meta_epoch_harness")
out2=$("$TMP/meta_epoch_harness")
echo "$out1"
echo "$out2"
e1=$(echo "$out1" | sed -n 's/.*epoch=\([-0-9]*\).*/\1/p')
e2=$(echo "$out2" | sed -n 's/.*epoch=\([-0-9]*\).*/\1/p')
if [[ -z "$e1" || -z "$e2" ]]; then
  echo "epoch parse fail" >&2
  exit 1
fi
if [[ "$e1" == "$e2" ]]; then
  echo "epoch fail: two processes matched ($e1)" >&2
  exit 1
fi
echo "OK cross-process epochs differ"
echo "meta_epoch_smoke: PASSED"
