#!/usr/bin/env bash
# job: P1 beta CLI surface on pure native oodac (tokens|ast|check|build|emit-c)
# fail-closed for out-of-surface commands. Dual-engine via chs_parity when native.
# in:  OODAC_BIN (default ./oodac/oodac), stage-0 ooda for parity
# out: exit 0 if beta surface rails green
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
OODA="${OODA:-$ROOT/bin/ooda}"

if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need native $OODAC (pure fixed_point stage-1)" >&2
  exit 1
fi
if [[ ! -x "$OODA" ]]; then
  echo "ERR_NO_OODA: need pure $OODA (scripts/bootstrap_no_cargo.sh)" >&2
  exit 1
fi

fail=0

# --- fail-closed out-of-surface ---
for bad in lsp run pkg migrate wasm llvm; do
  set +e
  "$OODAC" "$bad" "$ROOT/fixtures/int_main.oo" >"$TMPDIR/beta_bad.out" 2>"$TMPDIR/beta_bad.err"
  rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    echo "FAIL out-of-surface accepted: $bad" >&2
    fail=1
  else
    echo "OK fail-closed: $bad (exit=$rc)"
  fi
done

# --- tokens pass + fail ---
set +e
"$OODAC" tokens "$ROOT/fixtures/int_main.oo" >"$TMPDIR/beta_tok.out" 2>"$TMPDIR/beta_tok.err"
rt=$?
set -e
if [[ $rt -ne 0 ]] || ! grep -q $'\t' "$TMPDIR/beta_tok.out"; then
  echo "FAIL tokens pass int_main" >&2
  fail=1
else
  echo "OK tokens pass int_main"
fi
set +e
"$OODAC" tokens "$ROOT/bootstrap/corpus/lex/fail/bad_char.oo" >"$TMPDIR/beta_tokf.out" 2>"$TMPDIR/beta_tokf.err"
rtf=$?
set -e
if [[ $rtf -eq 0 ]]; then
  echo "FAIL tokens should fail-closed on bad_char" >&2
  fail=1
else
  echo "OK tokens fail-closed bad_char (exit=$rtf)"
fi

# --- ast pass ---
set +e
"$OODAC" ast "$ROOT/bootstrap/corpus/parse/pass/let_mut.oo" >"$TMPDIR/beta_ast.out" 2>"$TMPDIR/beta_ast.err"
ra=$?
set -e
if [[ $ra -ne 0 ]] || ! grep -q PROGRAM "$TMPDIR/beta_ast.out"; then
  echo "FAIL ast pass let_mut" >&2
  fail=1
else
  echo "OK ast pass let_mut"
fi

# --- check pass + fail ---
set +e
"$OODAC" check "$ROOT/bootstrap/corpus/check/pass/ok_main.oo" >"$TMPDIR/beta_ck.out" 2>"$TMPDIR/beta_ck.err"
rc0=$?
set -e
if [[ $rc0 -ne 0 ]]; then
  echo "FAIL check pass ok_main" >&2
  fail=1
else
  echo "OK check pass ok_main"
fi
set +e
"$OODAC" check "$ROOT/bootstrap/corpus/typecheck/fail/undefined_var.oo" >"$TMPDIR/beta_ckf.out" 2>"$TMPDIR/beta_ckf.err"
rc1=$?
set -e
if [[ $rc1 -eq 0 ]]; then
  echo "FAIL check must fail-closed undefined_var" >&2
  fail=1
else
  if ! grep -qE $'^ERR\ttype\t' "$TMPDIR/beta_ckf.out" "$TMPDIR/beta_ckf.err" 2>/dev/null; then
    echo "FAIL check missing ERR type" >&2
    fail=1
  else
    echo "OK check fail-closed undefined_var (exit=$rc1)"
  fi
fi
set +e
"$OODAC" check "$ROOT/bootstrap/corpus/check/fail/no_cap_fetch.oo" >"$TMPDIR/beta_cap.out" 2>"$TMPDIR/beta_cap.err"
rc2=$?
set -e
if [[ $rc2 -eq 0 ]]; then
  echo "FAIL check must fail-closed no_cap_fetch" >&2
  fail=1
else
  echo "OK check fail-closed no_cap_fetch (exit=$rc2)"
fi

# --- pure build CHS smoke (representative) ---
for rel in fixtures/chs_list_string.oo fixtures/while_count.oo bootstrap/corpus/emit-c/pass/println_int.oo; do
  src="$ROOT/$rel"
  base="$(basename "$rel" .oo)"
  bin="$TMPDIR/beta_build_${base}.bin"
  rm -f "$bin"
  set +e
  (cd "$ROOT" && "$OODAC" build "$src" "$bin") >"$TMPDIR/beta_build_${base}.log" 2>&1
  rb=$?
  set -e
  if [[ $rb -ne 0 ]] || [[ ! -x "$bin" ]]; then
    echo "FAIL pure build $rel" >&2
    head -8 "$TMPDIR/beta_build_${base}.log" >&2 || true
    fail=1
  else
    out=$("$bin" 2>/dev/null | tr -d '\r' | head -3 | tr '\n' ',')
    echo "OK pure build $rel -> $out"
  fi
done

# --- dual-engine (stage-0 vs native oodac) ---
set +e
(cd "$ROOT" && OODAC_MODE=native OODAC_BIN="$OODAC" OODA="$OODA" ./scripts/chs_parity.sh) >"$TMPDIR/beta_parity.log" 2>&1
rp=$?
set -e
if [[ $rp -ne 0 ]]; then
  echo "FAIL chs_parity native" >&2
  tail -20 "$TMPDIR/beta_parity.log" >&2 || true
  fail=1
else
  echo "OK chs_parity native (dual-engine tokens/ast/check)"
fi

if [[ $fail -ne 0 ]]; then
  echo "beta_cli_smoke: FAILED" >&2
  exit 1
fi
echo "beta_cli_smoke: PASSED"
exit 0
