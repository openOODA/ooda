#!/usr/bin/env bash
# job: QA probe suite — Chaos Pack (C1-C5) on compiled product binaries
# in:  bin/ooda, oodac/oodac
# out: exit 0 if all C1-C5 chaos probes pass without pipe masking
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd)" || true
[[ -n "$ROOT" && -d "$ROOT" ]] || { echo "ERR_ROOT_INVALID" >&2; exit 1; }

TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/probe_chaos_$$"
mkdir -p "$TMP"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

OODA="${OODA:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-}"
if [[ -z "$OODAC" || ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then OODAC="$ROOT/oodac/oodac"
  elif [[ -x "$ROOT/dist/ooda-v0.182.1-alpha-linux-x86_64/oodac/oodac" ]]; then OODAC="$ROOT/dist/ooda-v0.182.1-alpha-linux-x86_64/oodac/oodac"
  fi
fi
export OODAC_BIN="$OODAC"

fail=0
pass() { echo "OK [C-PROBE] $*"; }
bad() { echo "FAIL [C-PROBE] $*" >&2; fail=1; }

[[ -x "$OODA" ]] || { echo "ERR_MISSING_OODA: $OODA" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_MISSING_OODAC: $OODAC" >&2; exit 1; }

# C1: FD starvation (ulimit -n)
set +e
out1=$(ulimit -n 4; "$OODA" check "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc1=$?
set -e
if [[ $rc1 -ne 0 ]] && ! echo "$out1" | grep -qE '^OK$'; then
  pass "C1 FD starvation fail-closed (rc=$rc1)"
else
  bad "C1 FD starvation accepted / false OK (out=$out1)"
fi

# C2: Unwritable temp directory (TMPDIR=/proc)
set +e
out2=$(TMPDIR=/proc "$OODA" run "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc2=$?
set -e
if [[ $rc2 -ne 0 ]]; then
  pass "C2 unwritable TMPDIR fail-closed (rc=$rc2)"
else
  bad "C2 unwritable TMPDIR succeeded (out=$out2)"
fi

# C3: Corrupt & empty inputs
empty_f="$TMP/empty.oo"
touch "$empty_f"
set +e
out3_e=$("$OODA" check "$empty_f" 2>&1)
rc3_e=$?
set -e
if [[ $rc3_e -ne 0 ]] && echo "$out3_e" | grep -qE 'ERR.*empty'; then
  pass "C3 empty input rejected with ERR"
else
  bad "C3 empty input failure (rc=$rc3_e, out=$out3_e)"
fi

huge_f="$TMP/huge.oo"
head -c 70000 /dev/zero | tr '\0' 'a' > "$huge_f"
set +e
out3_h=$("$OODA" check "$huge_f" 2>&1)
rc3_h=$?
set -e
if [[ $rc3_h -ne 0 ]] && echo "$out3_h" | grep -qE '64KiB'; then
  pass "C3 oversize input (>64K) rejected"
else
  bad "C3 oversize input failure (rc=$rc3_h, out=$out3_h)"
fi

garb_f="$TMP/garbage.oo"
head -c 100 /dev/urandom > "$garb_f"
set +e
out3_g=$("$OODA" check "$garb_f" 2>&1)
rc3_g=$?
set -e
if [[ $rc3_g -ne 0 ]] && echo "$out3_g" | grep -qE 'ERR.*lex'; then
  pass "C3 binary garbage rejected"
else
  bad "C3 binary garbage failure (rc=$rc3_g, out=$out3_g)"
fi

# C4: Missing runtime include directory
set +e
out4=$(gcc -I/nonexistent_rt "$ROOT/runtime/chs_rt.c" "$ROOT/fixtures/test_100.oo.c" 2>&1)
rc4=$?
set -e
if [[ $rc4 -ne 0 ]]; then
  pass "C4 missing runtime headers gcc build fails"
else
  bad "C4 missing runtime headers build succeeded"
fi

# C5: Direct process exit status verification without pipe masking
set +e
"$OODAC" check "$ROOT/fixtures/chs_list_string.oo" >/dev/null 2>&1
rc5=$?
set -e
if [[ $rc5 -eq 0 ]]; then
  pass "C5 direct process exit verification without pipe masking"
else
  bad "C5 direct process check failed"
fi

if [[ $fail -ne 0 ]]; then
  echo "probe_chaos_tests: FAILED" >&2
  exit 1
fi
echo "probe_chaos_tests: PASSED"
exit 0
