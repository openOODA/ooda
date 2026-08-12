#!/usr/bin/env bash
# T3 DE1.8: sys_exec child env is filtered — OODA_/OO_ kept, secrets stripped.
# in:  runtime/chs_rt_sys.c + chs_rt umbrella
# out: exit 0 if compile + runtime filter semantics hold
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
SYS_C="$ROOT/runtime/chs_rt_sys.c"

# --- 1) source posture: prefix filter, no post-clearenv getenv for allow ---
if grep -q 'env_cap' "$SYS_C" \
  && grep -qE 'strncmp\(\*src, "OODA_"' "$SYS_C" \
  && grep -qE 'strncmp\(\*src, "OO_"' "$SYS_C"; then
  pass "source: env_cap + OODA_/OO_ prefix check on key"
else
  bad "source missing env_cap or prefix strncmp on *src"
fi
# Filter decision must not *call* oo_process_policy_getenv (uses getenv after clearenv).
# Comments may mention the bug; only flag a real call site outside the function def.
if grep -nE 'oo_process_policy_getenv\s*\(' "$SYS_C" \
  | grep -v 'const char \*oo_process_policy_getenv' \
  | grep -q .; then
  bad "non-def call to oo_process_policy_getenv in chs_rt_sys.c"
else
  pass "no post-clearenv oo_process_policy_getenv filter call"
fi
if grep -q 'clearenv' "$SYS_C" && grep -q 'PATH.*/usr/bin:/bin' "$SYS_C"; then
  pass "source: clearenv + minimal PATH"
else
  bad "missing clearenv or minimal PATH"
fi
# Real system(3) call only — comments like "no system(3)" are fine.
if grep -nE '[^[:alnum:]_]system\s*\(' "$SYS_C" | grep -vq 'system(3)'; then
  bad "sys.c uses system(3)"
else
  pass "no system(3); fork+execvp path"
fi

# --- 2) compile-only -Werror on owned TU ---
set +e
gcc -c -Werror -Wshadow -I"$ROOT/runtime" "$SYS_C" -o "$TMPDIR/chs_rt_sys.o" \
  2>"$TMPDIR/sys_c.gcc"
crc=$?
set -e
if [[ $crc -eq 0 ]]; then
  pass "gcc -c -Werror -Wshadow chs_rt_sys.c"
else
  bad "gcc -c -Werror chs_rt_sys.c"
  head -20 "$TMPDIR/sys_c.gcc" || true
fi

# --- 3) runtime: link harness, prove OODA_* kept / secrets not inherited ---
HARNESS="$TMPDIR/sys_exec_env_filter_harness.c"
BIN="$TMPDIR/sys_exec_env_filter_harness"
cat >"$HARNESS" <<'EOF'
#include "chs_rt.h"
/* Child prints markers; parent checks via wait status from oo_sys_exec. */
int main(void) {
  long long sys = oo_cap_grant_sys();
  OoStr av[3];
  OoResS r;
  /* Body: require OODA_MARK + OO_MARK; forbid secrets / common ambient. */
  av[0] = oo_str_lit("sh");
  av[1] = oo_str_lit("-c");
  av[2] = oo_str_lit(
      "test \"${OODA_MARK:-}\" = keepme || exit 10; "
      "test \"${OO_MARK:-}\" = keepoo || exit 11; "
      "test -z \"${AWS_SECRET_ACCESS_KEY:-}\" || exit 12; "
      "test -z \"${LD_PRELOAD:-}\" || exit 13; "
      "test -z \"${HOME:-}\" || exit 14; "
      "test -z \"${USER:-}\" || exit 15; "
      "case \"${PATH:-}\" in /usr/bin:/bin) ;; *) exit 16;; esac; "
      "if printenv 2>/dev/null | grep -qE '^(AWS_SECRET|LD_PRELOAD|HOME|USER)='; then exit 17; fi; "
      "exit 0");
  r = oo_sys_exec(sys, 3, av);
  if (!r.ok) {
    fprintf(stderr, "sys_exec failed (child non-zero or fork/exec error)\n");
    return 1;
  }
  return 0;
}
EOF

set +e
gcc -O0 -Werror -I"$ROOT/runtime" "$HARNESS" "$ROOT/runtime/chs_rt.c" \
  -lm -ldl -lpthread -o "$BIN" 2>"$TMPDIR/harness.gcc"
hrc=$?
set -e
if [[ $hrc -ne 0 ]]; then
  bad "link harness"
  head -25 "$TMPDIR/harness.gcc" || true
else
  pass "link env-filter harness"
  export OODA_MARK=keepme
  export OO_MARK=keepoo
  export AWS_SECRET_ACCESS_KEY=should_not_leak
  export LD_PRELOAD=
  # ensure ambient present in parent
  export HOME="${HOME:-/tmp}"
  export USER="${USER:-smoke}"
  # empty LD_PRELOAD is fine; set a sentinel that must not appear
  export AWS_SECRET_ACCESS_KEY="AKIA_SHOULD_NOT_LEAK"
  set +e
  "$BIN" >"$TMPDIR/harness.out" 2>"$TMPDIR/harness.err"
  rrc=$?
  set -e
  if [[ $rrc -eq 0 ]]; then
    pass "runtime: OODA_/OO_ kept; AWS/HOME/USER stripped; PATH minimal"
  else
    bad "runtime filter failed exit=$rrc"
    cat "$TMPDIR/harness.out" "$TMPDIR/harness.err" 2>/dev/null || true
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "sys_exec_env_filter_smoke: FAILED" >&2
  exit 1
fi
echo "sys_exec_env_filter_smoke: PASSED"
exit 0
