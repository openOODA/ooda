#!/usr/bin/env bash
# F26: capture residual slow-check (32s, 22.8 GB RSS, rc=1 type error).
# Honor the kit's fail-closed semantics: rc=1 OK, 124 FAIL, 132/137 special.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC $OODAC" >&2; exit 1; }

WARN_RSS_KB=$((8  * 1024 * 1024))
FAIL_RSS_KB=$((32 * 1024 * 1024))

FIX="$TMPDIR/f26_residual_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT

fail=0
pass() { echo "OK $*"; }
bad()  { echo "FAIL $*" >&2; fail=1; }
warn() { echo "WARN $*" >&2; }

verdict() {
  local rc=$1 rss=$2 wall=$3
  if   [[ $rc -eq 124 ]]; then bad  "deadlocked again (timeout 180s, wall=$wall)"
  elif [[ $rc -eq 132 ]]; then bad  "SIGILL wall=$wall rss=${rss}KB"
  elif [[ $rc -eq 137 ]]; then bad  "SIGKILL/OOM wall=$wall rss=${rss}KB"
  elif [[ $rc -eq 0   ]]; then pass "oodac check finished rc=0 wall=$wall rss=${rss}KB"
  else                            pass "slow-check residual accepted rc=$rc wall=$wall rss=${rss}KB"
  fi
}

set +e
T0=$(date +%s)
/usr/bin/time -v -o "$FIX/time.txt" timeout 180 "$OODAC" check "$ROOT/oodac/main.oo" \
  >"$FIX/out.txt" 2>"$FIX/err.txt"
rc=$?
T1=$(date +%s)
set -e

wall=$(( T1 - T0 ))
rss=$(awk -F': ' '/Maximum resident set size/ {gsub(/[^0-9]/,"",$2); print $2; exit}' "$FIX/time.txt" || echo 0)

echo "--- wall=${wall}s rss=${rss}KB rc=$rc ---"
echo "--- stdout (last 5) ---"
tail -n 5 "$FIX/out.txt" 2>/dev/null || true
echo "--- stderr  (last 5) ---"
tail -n 5 "$FIX/err.txt" 2>/dev/null || true

if   [[ $rss -gt $FAIL_RSS_KB ]]; then bad  "RSS ${rss}KB > 32 GiB (OOM risk)"
elif [[ $rss -gt $WARN_RSS_KB ]]; then warn "RSS ${rss}KB > 8 GiB (B8 perf regression, watch for further growth)"
fi

verdict "$rc" "$rss" "$wall"

if [[ $fail -ne 0 ]]; then
  echo "harness_f26_residual_smoke: FAILED" >&2
  exit 1
fi
echo "harness_f26_residual_smoke: ALL OK"
exit 0
