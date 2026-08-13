#!/usr/bin/env bash
# Prove load-time hide: ctor runs before main; table is once-per-process.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCR="${OODA_HIDE_SCRATCH:-$ROOT/../.ooda-hide-smoke}"
mkdir -p "$SCR"
cat >"$SCR/hide_harness.c" <<'C'
#include "oo_hide.h"
#include <stdio.h>
int main(void) {
  unsigned long long t1[OO_HIDE_SLOTS];
  unsigned long long t2[OO_HIDE_SLOTS];
  unsigned long long fold;
  unsigned long long fp1;
  unsigned long long fp2;
  int i;
  int en;
  if (!oo_hide_loaded_at_start() || !oo_hide_ready()) {
    fprintf(stderr, "ctor did not fill table before main\n");
    return 1;
  }
  en = oo_hide_enabled();
  oo_hide_table(t1, OO_HIDE_SLOTS);
  oo_hide_table(t2, OO_HIDE_SLOTS);
  fp1 = oo_hide_fingerprint();
  fp2 = oo_hide_fingerprint();
  fold = 0;
  for (i = 0; i < OO_HIDE_SLOTS; i++) {
    if (t1[i] != t2[i]) {
      fprintf(stderr, "same-process table resampled\n");
      return 1;
    }
    fold ^= t1[i] + (unsigned long long)(i + 1);
  }
  if (fp1 != fp2 || fp1 != fold) {
    fprintf(stderr, "fingerprint is not fold of load table\n");
    return 1;
  }
  if (!en) {
    for (i = 0; i < OO_HIDE_SLOTS; i++) {
      if (t1[i] != 0) {
        fprintf(stderr, "hide-off table not zeros\n");
        return 1;
      }
    }
  } else {
    for (i = 0; i < OO_HIDE_SLOTS; i++) {
      if (t1[i] == 0) {
        fprintf(stderr, "hide-on zero slot\n");
        return 1;
      }
    }
  }
  printf("loaded=1 enabled=%d slots=", en);
  for (i = 0; i < OO_HIDE_SLOTS; i++) {
    if (i) {
      printf(",");
    }
    printf("%llx", t1[i]);
  }
  printf(" fp=%llu\n", (unsigned long long)fp1);
  return 0;
}
C
gcc -O0 -I"$ROOT/runtime" "$SCR/hide_harness.c" "$ROOT/runtime/oo_hide.c" -o "$SCR/hide_harness"
DISK1=$(sha256sum "$SCR/hide_harness" | awk '{print $1}')
unset OODA_HIDE || true
offu1=$(env -u OODA_HIDE "$SCR/hide_harness")
offu2=$(env -u OODA_HIDE "$SCR/hide_harness")
echo "unset $offu1"
echo "unset $offu2"
if [[ "$offu1" != "$offu2" ]] || ! echo "$offu1" | grep -q 'enabled=0'; then
  echo "FAIL unset hide-off not stable zeros" >&2
  exit 1
fi
if ! echo "$offu1" | grep -q 'loaded=1'; then
  echo "FAIL ctor flag missing on unset" >&2
  exit 1
fi
off1=$(OODA_HIDE=0 "$SCR/hide_harness")
off2=$(OODA_HIDE=0 "$SCR/hide_harness")
echo "$off1"
echo "$off2"
if [[ "$off1" != "$off2" ]] || ! echo "$off1" | grep -q 'enabled=0'; then
  echo "FAIL hide-off not stable" >&2
  exit 1
fi
on1=$(OODA_HIDE=1 "$SCR/hide_harness")
on2=$(OODA_HIDE=1 "$SCR/hide_harness")
echo "$on1"
echo "$on2"
if [[ "$on1" == "$on2" ]]; then
  echo "FAIL hide-on two processes matched" >&2
  exit 1
fi
if ! echo "$on1" | grep -q 'enabled=1' || ! echo "$on2" | grep -q 'enabled=1'; then
  echo "FAIL hide-on not enabled" >&2
  exit 1
fi
s1=${on1#*slots=}
s2=${on2#*slots=}
s1=${s1%% fp=*}
s2=${s2%% fp=*}
if [[ -z "$s1" || "$s1" == "$s2" ]]; then
  echo "FAIL hide-on slot dump did not differ" >&2
  exit 1
fi
DISK2=$(sha256sum "$SCR/hide_harness" | awk '{print $1}')
if [[ "$DISK1" != "$DISK2" ]]; then
  echo "FAIL on-disk binary changed" >&2
  exit 1
fi
echo "disk=$DISK1"
echo "oo_hide_smoke: PASSED"
