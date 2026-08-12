#!/usr/bin/env bash
# job: 7.2 c_sys_exec_last_arg path-A smoke
# in:  OODAC_BIN (default oodac/oodac) + ephemeral TMPDIR fixtures
# out: exit 0 only if fail-closed + no-FP rails hold
#
# Rails:
#   PASS fail-closed — // SECRET: tok then sys_exec last arg is bare IDENT tok
#   PASS no-FP       — quoted last arg containing commas + secret-looking text
#
# Residual: full IFC / #[Secret] / interp ${} / 7.10 parse-env char concat.
# Does not rewrite Domain Expert product logic (c_emit_args.oo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

FIX="$TMPDIR/s72_sys_exec_last_arg_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT

# --- fixtures (all under TMPDIR; never pollute repo root) ---

# FAIL-CLOSED: SECRET ident as last sys_exec arg (beyond a1/a2) must refuse
cat >"$FIX/sys_exec_last_ident.oo" <<'EOF'
// 7.2 path-A: bare SECRET IDENT as last sys_exec argv must refuse
// SECRET: tok
pub fn main(sys: &SysCap) {
    let tok = "s3cret";
    let r = sys_exec(sys, "sh", "-c", tok);
    println(1);
}
EOF

# NO-FP: quoted last arg with commas + secret-looking text must not refuse
cat >"$FIX/sys_exec_last_quoted.oo" <<'EOF'
// 7.2 path-A: last argv is a string literal (commas + SECRET spelling) — no FP
// SECRET: tok
pub fn main(sys: &SysCap) {
    let r = sys_exec(sys, "sh", "-c", "echo hello, tok, SECRET");
    println(1);
}
EOF

# --- helpers ---

expect_refuse() {
  local src="$1"
  local label="$2"
  local out="$FIX/${label}.out"
  set +e
  "$OODAC" emit-c "$src" >"$out" 2>&1
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    bad "fail-closed accepted (want ERR secret): $label"
    head -8 "$out" >&2 || true
    return 1
  fi
  if ! grep -qE $'ERR\tsecret' "$out"; then
    bad "fail-closed missing ERR secret: $label (got: $(head -2 "$out"))"
    return 1
  fi
  pass "fail-closed: $label"
  return 0
}

expect_ok() {
  local src="$1"
  local label="$2"
  local out="$FIX/${label}.out"
  set +e
  "$OODAC" emit-c "$src" >"$out" 2>&1
  local rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    bad "no-FP rail refused (want OK): $label rc=$rc"
    head -8 "$out" >&2 || true
    return 1
  fi
  if grep -qE $'ERR\tsecret' "$out"; then
    bad "no-FP rail has ERR secret: $label"
    return 1
  fi
  pass "no-FP: $label"
  return 0
}

# --- rails ---

expect_refuse "$FIX/sys_exec_last_ident.oo" "secret_tok_sys_exec_last_ident" || true
expect_ok "$FIX/sys_exec_last_quoted.oo" "secret_tok_sys_exec_last_quoted_commas" || true

if [[ $fail -ne 0 ]]; then
  echo "s72_sys_exec_last_arg_smoke: FAILED" >&2
  exit 1
fi
echo "s72_sys_exec_last_arg_smoke: ALL OK"
echo "RESIDUAL full IFC / #[Secret] attr / string-interp \${} deep taint — not claimed closed"
echo "RESIDUAL 7.10: c_parse_secret_env still char-by-char name concat (not this OWN)"
echo "RESIDUAL last-arg slice hygiene is product (c_sys_exec_last_arg); this smoke proves emit rails only"
exit 0
