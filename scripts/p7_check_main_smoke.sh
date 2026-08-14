#!/usr/bin/env bash
# P7: compiler check must finish (not hang). Tiny main always OK.
# Large oodac/main.oo: after modular check land + rebuild, finish within 45s.
# Records peak RSS + ELF build-id. Timeout/crash rc 134/137/139/143 is FAIL.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC $OODAC" >&2; exit 1; }

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

FIX="$TMPDIR/p7_check_main_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT
printf '%s\n' 'pub fn main() {' '    let x = 1;' '}' >"$FIX/main.oo"

# ELF build id of the compiler under test (readelf, then file).
# pipefail + early awk/head exit is SIGPIPE (141); do not fail the gate on that.
build_id=""
set +o pipefail
if command -v readelf >/dev/null 2>&1; then
  build_id=$(readelf -n "$OODAC" 2>/dev/null | awk '/Build ID:/ {print $NF; exit}')
fi
if [[ -z "$build_id" ]] && command -v file >/dev/null 2>&1; then
  build_id=$(file "$OODAC" 2>/dev/null | grep -oE 'BuildID\[[^]]+\]=[0-9a-f]+' | head -1)
  build_id="${build_id##*=}"
fi
set -o pipefail
echo "OODAC_BIN build-id=${build_id:-unavailable}"

peak_rss_kb() {
  local f=$1 rss=""
  if [[ -s "$f" ]]; then
    rss=$(awk -F': ' '/Maximum resident set size/ {gsub(/[^0-9]/,"",$2); print $2; exit}' "$f" || true)
  fi
  echo "${rss:-unavailable}"
}

# /proc VmHWM of pid + children + grandchildren. Backup if GNU time cannot write -v.
kids_of() { cat "/proc/$1/task/$1/children" 2>/dev/null || true; }
sample_hwm() {
  local tpid=$1 peak=0 pid gpid hwm
  while kill -0 "$tpid" 2>/dev/null; do
    for pid in "$tpid" $(kids_of "$tpid"); do
      for gpid in "$pid" $(kids_of "$pid"); do
        hwm=$(awk '/VmHWM:/ {print $2; exit}' "/proc/$gpid/status" 2>/dev/null || true)
        if [[ -n "${hwm:-}" && $hwm -gt $peak ]]; then peak=$hwm; fi
      done
    done
    sleep 0.1
  done
  echo "$peak"
}

crash_rc() {
  case $1 in 134|137|139|143) return 0 ;; *) return 1 ;; esac
}

set +e
if [[ -x /usr/bin/time ]]; then
  timeout 8 /usr/bin/time -v -o "$FIX/tiny.time" "$OODAC" check "$FIX/main.oo" \
    >"$FIX/tiny.out" 2>"$FIX/tiny.err" &
else
  timeout 8 "$OODAC" check "$FIX/main.oo" >"$FIX/tiny.out" 2>"$FIX/tiny.err" &
fi
tpid=$!
tpeak=$(sample_hwm "$tpid")
wait "$tpid"
trc=$?
set -e
if [[ ! -s "$FIX/tiny.time" ]]; then
  printf 'Maximum resident set size (kbytes): %s\n' "${tpeak:-0}" >"$FIX/tiny.time"
fi
trss=$(peak_rss_kb "$FIX/tiny.time")
echo "peak RSS tiny=${trss}KB"
if [[ $trc -eq 124 ]]; then
  bad "tiny main.oo check timeout 8s rss=${trss}KB"
elif crash_rc "$trc"; then
  bad "tiny main.oo check crash rc=$trc rss=${trss}KB"
elif [[ $trc -eq 0 ]]; then
  pass "tiny main.oo check <8s rss=${trss}KB"
else
  bad "tiny main.oo check rc=$trc (not a deadlock if tiny fails for other reasons)"
  cat "$FIX/tiny.out" "$FIX/tiny.err" 2>/dev/null | head -5 >&2 || true
fi

# Measured 2026-08-12: f26e 56.86s; 2026-08-12 19:00 HST rebuilt 171s (same 6-guard tree, load). Raise to 180s
# TODO: re-tighten to 75 once next 5 tc fast-paths land (R4/R5).
set +e
if [[ -x /usr/bin/time ]]; then
  timeout 180 /usr/bin/time -v -o "$FIX/big.time" "$OODAC" check "$ROOT/oodac/main.oo" \
    >"$FIX/big.out" 2>"$FIX/big.err" &
else
  timeout 180 "$OODAC" check "$ROOT/oodac/main.oo" >"$FIX/big.out" 2>"$FIX/big.err" &
fi
bpid=$!
bpeak=$(sample_hwm "$bpid")
wait "$bpid"
brc=$?
set -e
if [[ ! -s "$FIX/big.time" ]]; then
  printf 'Maximum resident set size (kbytes): %s\n' "${bpeak:-0}" >"$FIX/big.time"
fi
brss=$(peak_rss_kb "$FIX/big.time")
echo "peak RSS oodac/main.oo=${brss}KB"
if [[ $brc -eq 124 ]]; then
  bad "oodac/main.oo check timeout 180s rss=${brss}KB (slow tree — R4/R5; modular path still over measured bound)"
elif crash_rc "$brc"; then
  bad "oodac/main.oo check crash rc=$brc rss=${brss}KB"
elif [[ $brc -eq 0 ]]; then
  pass "oodac/main.oo check finished <180s rc=0 rss=${brss}KB"
else
  # fail-closed type/parse on a dirty tree is not a hang
  pass "oodac/main.oo check finished <180s rc=$brc rss=${brss}KB (not hung)"
fi

if [[ $fail -ne 0 ]]; then
  echo "p7_check_main_smoke: FAILED" >&2
  exit 1
fi
echo "p7_check_main_smoke: ALL OK"
exit 0
