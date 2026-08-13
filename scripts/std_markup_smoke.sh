#!/usr/bin/env bash
# job: pure std/markup path A check + multi-build fixture
# in:  oodac, std/markup/*.oo, fixtures/std_markup_main.oo
# out: exit 0 if markup path A is real on pure path
# residual: full XML/YAML/TOML/JSON Schema NOT product
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

[[ -x "$OODAC" ]] || { echo ERR_NO_OODAC >&2; exit 1; }

for m in toml.oo yaml.oo xml.oo json_schema.oo; do
  set +e
  "$OODAC" check "$ROOT/std/markup/$m" \
    >"$TMPDIR/std_mk_ck_$m.out" 2>"$TMPDIR/std_mk_ck_$m.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    bad "check std/markup/$m"
    head -12 "$TMPDIR/std_mk_ck_$m.err" 2>/dev/null || true
    head -12 "$TMPDIR/std_mk_ck_$m.out" 2>/dev/null || true
  else
    pass "check std/markup/$m"
  fi
done

# Residual honesty: no DESIGN-done / full-parser claims
for m in toml.oo yaml.oo xml.oo json_schema.oo; do
  f="$ROOT/std/markup/$m"
  if grep -qiE 'full (XML|YAML|TOML|JSON Schema) parser|DESIGN.?done' "$f"; then
    bad "honesty std/markup/$m forbidden claim"
  else
    if grep -qiE 'Not full|UNIMPLEMENTED_RESIDUAL|NOT product|not full' "$f"; then
      pass "honesty residual std/markup/$m"
    else
      bad "honesty residual missing std/markup/$m"
    fi
  fi
done

f="$ROOT/fixtures/std_markup_main.oo"
set +e
OODAC_BIN="$OODAC" "$OODAC" build "$f" "$TMPDIR/std_markup_main" \
  >"$TMPDIR/std_mk_b.out" 2>"$TMPDIR/std_mk_b.err"
rc=$?
set -e
if [[ $rc -ne 0 || ! -x "$TMPDIR/std_markup_main" ]]; then
  bad "build fixtures/std_markup_main.oo"
  head -20 "$TMPDIR/std_mk_b.err" 2>/dev/null || true
  head -20 "$TMPDIR/std_mk_b.out" 2>/dev/null || true
else
  set +e
  "$TMPDIR/std_markup_main" >"$TMPDIR/std_mk_r.out" 2>"$TMPDIR/std_mk_r.err"
  rr=$?
  set -e
  out="$(cat "$TMPDIR/std_mk_r.out" 2>/dev/null || true)"
  if [[ $rr -ne 0 ]]; then
    bad "run fixtures/std_markup_main.oo rc=$rr"
    head -12 "$TMPDIR/std_mk_r.err" 2>/dev/null || true
  elif ! echo "$out" | grep -q $'name\tada'; then
    bad "run missing name<TAB>ada (got: $out)"
  elif ! echo "$out" | grep -q 'toml-residual'; then
    bad "run missing toml-residual (got: $out)"
  elif ! echo "$out" | grep -q 'yaml-residual'; then
    bad "run missing yaml-residual (got: $out)"
  elif ! printf '%s\n' "$out" | grep -Fxq 'yo'; then
    bad "run missing xml strip yo (got: $out)"
  elif ! echo "$out" | grep -q 'xml-residual'; then
    bad "run missing xml-residual (got: $out)"
  elif ! echo "$out" | grep -q 'js-empty'; then
    bad "run missing js-empty (got: $out)"
  elif ! echo "$out" | grep -q 'js-residual'; then
    bad "run missing js-residual (got: $out)"
  elif ! echo "$out" | grep -q 'js-validate-deny'; then
    bad "run missing js-validate-deny (got: $out)"
  else
    pass "build+run fixtures/std_markup_main.oo"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "std_markup_smoke: FAILED" >&2
  exit 1
fi
echo "std_markup_smoke: PASSED"
exit 0
