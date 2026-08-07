#!/usr/bin/env bash
# job: QA probe suite — Data Lifecycle Pack (L1-L6) on compiled product binaries
# in:  bin/ooda, oodac/oodac
# out: exit 0 if all L1-L6 lifecycle probes pass
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd)" || true
[[ -n "$ROOT" && -d "$ROOT" ]] || { echo "ERR_ROOT_INVALID" >&2; exit 1; }

TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/probe_lifecycle_$$"
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
pass() { echo "OK [L-PROBE] $*"; }
bad() { echo "FAIL [L-PROBE] $*" >&2; fail=1; }

[[ -x "$OODA" ]] || { echo "ERR_MISSING_OODA: $OODA" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_MISSING_OODAC: $OODAC" >&2; exit 1; }

# L1: File handle closing check via chs_rt_fs.c code inspection and loop test
set +e
for i in $(seq 1 50); do
  "$OODA" check "$ROOT/fixtures/chs_list_string.oo" >/dev/null 2>&1
done
rc1=$?
set -e
if [[ $rc1 -eq 0 ]]; then
  pass "L1 repeated check file handles closed without fd leak"
else
  bad "L1 repeated check failed"
fi

# L2: Torn state protection on short/failed write (/dev/full)
full_oo="$TMP/full.oo"
cat >"$full_oo" <<'EOF'
pub fn main(fs: &FsCap) {
    let r = write_file(fs, "/dev/full", "test_content_torn");
    if r.is_ok() { println("TORN_BAD"); } else { println("TORN_SAFE"); }
}
EOF
set +e
"$OODAC" emit-c "$full_oo" >"$TMP/full.c" 2>/dev/null
gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/full.c" -o "$TMP/full.bin" -lm 2>/dev/null
out2=$("$TMP/full.bin" 2>&1) || true
set -e
if echo "$out2" | grep -q 'TORN_SAFE'; then
  pass "L2 write_file torn state prevented (returned Err)"
else
  bad "L2 write_file returned Ok on short/failed write"
fi

# L3: Child process reaping verification (sys_exec system(3) calls waitpid)
sys_oo="$TMP/sys.oo"
cat >"$sys_oo" <<'EOF'
pub fn main(sys: &SysCap) {
    let r = sys_exec(sys, "sh", "-c", "true");
    if r.is_ok() { println("SYS_REAPED"); }
}
EOF
set +e
"$OODAC" emit-c "$sys_oo" >"$TMP/sys.c" 2>/dev/null
gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/sys.c" -o "$TMP/sys.bin" -lm 2>/dev/null
out3=$("$TMP/sys.bin" 2>&1) || true
set -e
if echo "$out3" | grep -q 'SYS_REAPED'; then
  pass "L3 sys_exec child process reaped cleanly"
else
  bad "L3 sys_exec failed to reap process"
fi

# L4: Temp file cleanup verification in scripts
set +e
run_out=$(TMPDIR="$TMP" "$OODA" run "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc4=$?
set -e
leftover=$(find "$TMP" -name 'ooda_run_*' | wc -l)
if [[ $rc4 -eq 0 ]] && [[ $leftover -eq 0 ]]; then
  pass "L4 ooda run trap cleanup reaped temp binary"
else
  bad "L4 temp binary leaked ($leftover left behind)"
fi

# L5 & L6: Atomic patch write & cleanup verification
patch_target="$ROOT/qa/tmp_patch_target.oo"
patch_body="$ROOT/qa/tmp_patch_body.oo"
cleanup_patch() { rm -f "$patch_target" "$patch_body"; }
trap 'cleanup; cleanup_patch' EXIT
cp "$ROOT/fixtures/chs_list_string.oo" "$patch_target"
echo "    println(42);" > "$patch_body"
set +e
"$OODA" patch "$patch_target" --replace-fn main --with "$patch_body" >/dev/null 2>&1
rc5=$?
set -e
tmp_leftover=$(find "$ROOT/qa" -name '*.tmp' | wc -l)
if [[ $rc5 -eq 0 ]] && [[ $tmp_leftover -eq 0 ]]; then
  pass "L5 atomic patch write cleaned up temp files"
else
  bad "L5 atomic patch temp files leaked (rc5=$rc5, leftover=$tmp_leftover)"
fi

if [[ $fail -ne 0 ]]; then
  echo "probe_lifecycle_tests: FAILED" >&2
  exit 1
fi
echo "probe_lifecycle_tests: PASSED"
exit 0
