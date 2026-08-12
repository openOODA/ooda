#!/usr/bin/env bash
# job: 2.8 macro-body SECRET second pass path-A smoke
# in:  OODAC_BIN (default oodac/oodac) + ephemeral TMPDIR fixtures
# out: exit 0 only if rails honest
#
# Rails:
#   macros unsupported:
#     FAIL-closed — check residual-refuse free-name macro_expand / ast_macro
#                   OR parse/type refuse of invented macro syntax (not silent green)
#     N/A         — SECRET-in-expanded-body not claimed; residual doc names refuse
#   macros exist:
#     FAIL-closed — SECRET ident in expanded body must ERR secret
#
# Residual: full IFC / #[Secret] / interp ${} / second-pass walker
#           (unneeded while AST_MACROS residual deny holds).
#           emit-llvm rc=0 call @macro_expand is named residual (not expand).
# Reopen: deleting macro_expand/ast_macro from residual_feature_of without
#         a post-expand SECRET walk in the same change is a regression.
# Does not rewrite Domain Expert product (check_residual.oo / c_emit_secret*).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }

fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
FIX="$TMPDIR/s28_macro_secret_$$"
mkdir -p "$FIX"
trap 'rm -rf "$FIX"' EXIT
LAST_RC=0
run_cmd() {
  set +e
  "$OODAC" "$1" "$2" >"$3" 2>&1
  LAST_RC=$?
  set -e
}

# --- fixtures (TMPDIR only) ---
cat >"$FIX/macro_expand.oo" <<'EOF'
pub fn main() { let r = macro_expand("m"); }
EOF
cat >"$FIX/ast_macro.oo" <<'EOF'
pub fn main() { let r = ast_macro("m"); }
EOF
cat >"$FIX/macro_kw.oo" <<'EOF'
macro dump(x) { println(x); }
pub fn main() { dump("s"); }
EOF
cat >"$FIX/secret_println.oo" <<'EOF'
// SECRET: tok
pub fn main() { let tok = "s3cret"; println(tok); }
EOF
cat >"$FIX/secret_macro_expand_body.oo" <<'EOF'
// SECRET: tok
pub fn main() { let tok = "s3cret"; let r = macro_expand("println(tok)"); }
EOF
cat >"$FIX/secret_ast_macro_body.oo" <<'EOF'
// SECRET: tok
pub fn main() { let tok = "s3cret"; let r = ast_macro("println(tok)"); }
EOF
cat >"$FIX/secret_macro_kw_body.oo" <<'EOF'
// SECRET: tok
macro leak(x) { println(x); }
pub fn main() { let tok = "s3cret"; leak(tok); }
EOF

is_residual() { grep -qE $'ERR\tresidual' "$1" && grep -qF "$2" "$1" && grep -qE 'AST_MACROS' "$1"; }
is_secret() { grep -qE $'ERR\tsecret' "$1"; }

expect_not_ok() {
  local cmd="$1" src="$2" label="$3"
  run_cmd "$cmd" "$src" "$FIX/${label}.out"
  if [[ $LAST_RC -eq 0 ]]; then
    bad "fail-closed $cmd accepted: $label"
    head -6 "$FIX/${label}.out" >&2 || true
    return 1
  fi
  pass "fail-closed $cmd: $label"
}

expect_residual_check() {
  local src="$1" name="$2" label="$3"
  local out="$FIX/${label}.out"
  run_cmd check "$src" "$out"
  if [[ $LAST_RC -eq 0 ]]; then
    bad "residual-refuse accepted: $label"; return 1
  fi
  if is_residual "$out" "$name"; then
    pass "residual-refuse check $name: $label"; return 0
  fi
  if grep -qE $'ERR\t(parse|type|c_emit)' "$out"; then
    pass "syntax-refuse check $name: $label"; return 0
  fi
  bad "check refuse missing residual/syntax ERR: $label"
}

expect_secret_refuse() {
  local src="$1" label="$2" cmd out
  for cmd in check emit-c; do
    out="$FIX/${label}.${cmd}.out"
    run_cmd "$cmd" "$src" "$out"
    if [[ $LAST_RC -eq 0 ]]; then
      bad "SECRET expanded-body $cmd accepted: $label"; continue
    fi
    if ! is_secret "$out"; then
      bad "SECRET expanded-body $cmd missing ERR secret: $label"; continue
    fi
    pass "SECRET expanded-body $cmd refuse: $label"
  done
}

expect_secret_floor() {
  run_cmd emit-c "$1" "$FIX/${2}.out"
  if [[ $LAST_RC -eq 0 ]] || ! is_secret "$FIX/${2}.out"; then
    bad "SECRET floor missing ERR secret: $2"; return 1
  fi
  pass "SECRET floor refuse: $2"
}

document_na() {
  echo "N/A rail: SECRET-in-expanded-body not claimed ($1)"
  echo "N/A rail: product refuses residual free-name macro_expand / ast_macro (AST_MACROS)"
  echo "N/A rail: second-pass SECRET walker unneeded while residual deny holds"
}

expect_residual_doc() {
  local doc="$ROOT/bootstrap/MACRO_SECRET_RESIDUAL.oot"
  [[ -f "$doc" ]] || doc="$ROOT/bootstrap/MACRO_SECRET_RESIDUAL.md"
  if [[ ! -f "$doc" ]]; then
    bad "N/A rail undocumented: missing MACRO_SECRET_RESIDUAL.oot"; return 1
  fi
  grep -q 'MACRO_SECRET_RESIDUAL_ALPHA' "$doc" || { bad "doc missing MACRO_SECRET_RESIDUAL_ALPHA"; return 1; }
  if grep -nEi 'second pass (is |is now )?(shipped|enforced|product.?green)' "$doc" \
    | grep -viE 'not |residual|do not|unneeded|never' >/dev/null; then
    bad "residual doc claims second pass shipped/enforced"; return 1
  fi
  grep -qE 'macro_expand|ast_macro|residual' "$doc" || { bad "doc does not name residual refuse"; return 1; }
  pass "N/A rail documented: $(basename "$doc")"
}

# --- classify ---
MACROS=0
ACCEPTED=""
for pair in "macro_expand:$FIX/macro_expand.oo" "ast_macro:$FIX/ast_macro.oo" "macro_kw:$FIX/macro_kw.oo"; do
  name="${pair%%:*}"; src="${pair#*:}"
  run_cmd check "$src" "$FIX/probe_${name}.out"
  if [[ $LAST_RC -eq 0 ]]; then
    MACROS=1; ACCEPTED="$ACCEPTED $name"
    pass "probe: check accepted $name (macros exist)"
  else
    pass "probe: check refused $name (rc=$LAST_RC)"
  fi
done

expect_secret_floor "$FIX/secret_println.oo" "secret_tok_println" || true

if [[ $MACROS -eq 0 ]]; then
  expect_residual_check "$FIX/macro_expand.oo" "macro_expand" "residual_macro_expand" || true
  expect_residual_check "$FIX/ast_macro.oo" "ast_macro" "residual_ast_macro" || true
  expect_not_ok check "$FIX/macro_kw.oo" "macro_kw_syntax" || true
  expect_not_ok emit-c "$FIX/macro_expand.oo" "emit_macro_expand" || true
  expect_not_ok emit-c "$FIX/ast_macro.oo" "emit_ast_macro" || true
  expect_not_ok emit-c "$FIX/macro_kw.oo" "emit_macro_kw" || true
  if is_residual "$FIX/residual_macro_expand.out" "macro_expand" \
    && is_residual "$FIX/residual_ast_macro.out" "ast_macro"; then
    expect_residual_doc || true
    document_na "residual free-name refuse"
    pass "macros unsupported: residual N/A rail honest"
  else
    document_na "fail-closed macro syntax (no expansion)"
    pass "macros unsupported: syntax fail-closed (N/A SECRET-in-body)"
  fi
else
  echo "macros exist:$ACCEPTED — SECRET in expanded body must refuse"
  [[ "$ACCEPTED" == *macro_expand* ]] && expect_secret_refuse "$FIX/secret_macro_expand_body.oo" "secret_in_macro_expand_body"
  [[ "$ACCEPTED" == *ast_macro* ]] && expect_secret_refuse "$FIX/secret_ast_macro_body.oo" "secret_in_ast_macro_body"
  [[ "$ACCEPTED" == *macro_kw* ]] && expect_secret_refuse "$FIX/secret_macro_kw_body.oo" "secret_in_macro_kw_body"
fi

# --- prove no-expand (I2): static + string-body + llvm honesty ---
hits=$(grep -lE 'macro_expand|ast_macro' "$ROOT/oodac"/*.oo 2>/dev/null || true)
if echo "$hits" | grep -q 'check_residual.oo' && [[ $(echo "$hits" | grep -c . || true) -eq 1 ]]; then
  pass "static: residual names only in check_residual.oo"
else
  bad "static: unexpected macro_expand/ast_macro in oodac: $hits"
fi

run_cmd check "$FIX/secret_macro_expand_body.oo" "$FIX/string_body_check.out"
if [[ $LAST_RC -eq 0 ]]; then
  bad "string-body check accepted (want residual refuse)"
elif is_secret "$FIX/string_body_check.out"; then
  bad "string-body check false SECRET (would imply expand)"
elif is_residual "$FIX/string_body_check.out" "macro_expand"; then
  pass "string-body not secret (residual only; no expand)"
else
  bad "string-body unexpected refuse: $(tr -d '\n' <"$FIX/string_body_check.out" | head -c 160)"
fi

run_cmd emit-llvm "$FIX/macro_expand.oo" "$FIX/llvm_macro.out"
if [[ $LAST_RC -eq 0 ]]; then
  if grep -q '@macro_expand' "$FIX/llvm_macro.out" && ! grep -q 'call void @oo_println' "$FIX/llvm_macro.out"; then
    pass "emit-llvm no-expand (named residual: rc=0 call @macro_expand)"
  else
    bad "emit-llvm unexpected expand/shape"
  fi
elif grep -qE $'ERR\tresidual' "$FIX/llvm_macro.out"; then
  pass "emit-llvm residual-refuse"
else
  pass "emit-llvm fail-closed rc=$LAST_RC (not expand)"
fi

if [[ $fail -ne 0 ]]; then
  echo "s28_macro_secret_smoke: FAILED" >&2
  exit 1
fi
echo "s28_macro_secret_smoke: ALL OK"
echo "RESIDUAL full IFC / #[Secret] attr / string-interp \${} deep taint — not claimed closed"
echo "RESIDUAL 2.8 second-pass walker: unneeded while AST_MACROS residual deny holds"
echo "RESIDUAL emit-c residual-deny is check-path; emit-c unknown-free-call is not expansion"
echo "RESIDUAL emit-llvm rc=0 call @macro_expand is link residual (no expand; not 2.8 SECRET FN)"
exit 0
