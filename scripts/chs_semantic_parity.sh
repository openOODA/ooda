#!/usr/bin/env bash
# ===================================================================
# CHS dual-engine semantic parity: `ooda run` vs `ooda build --target c`
# Compares normalized stdout digests. Fail closed on diverge.
# ===================================================================
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODA="${OODA_BIN:-$ROOT/target/release/ooda}"
if [[ ! -x "$OODA" ]]; then
  echo "error: need $OODA (cargo build --release) or OODA_BIN" >&2
  exit 1
fi

normalize() {
  # Keep pure numeric lines only (drop 🧪 / 🚀 banners)
  grep -E '^[0-9]+$' "$1" 2>/dev/null | tr '\n' ',' | sed 's/,$//' || true
}

EXAMPLES=(
  "examples/while_count.oo"
  "examples/int_main.oo"
  "examples/chs_hello.oo"
)

fail=0
for rel in "${EXAMPLES[@]}"; do
  src="$ROOT/$rel"
  if [[ ! -f "$src" ]]; then
    echo "SKIP missing $rel"
    continue
  fi
  base="$(basename "$rel" .oo)"
  run_out="$TMPDIR/chs_run_${base}.txt"
  bin_txt="$TMPDIR/chs_bin_${base}.txt"
  bin_path="${src%.oo}"

  rm -f "$bin_path" "$run_out" "$bin_txt"
  if ! "$OODA" run "$src" >"$run_out" 2>/dev/null; then
    echo "FAIL run $rel"
    fail=$((fail + 1))
    continue
  fi
  if ! "$OODA" build --target c "$src" >/dev/null 2>&1; then
    echo "FAIL build-c $rel"
    fail=$((fail + 1))
    continue
  fi
  if [[ ! -x "$bin_path" ]]; then
    echo "FAIL no binary for $rel at $bin_path"
    fail=$((fail + 1))
    continue
  fi
  if ! "$bin_path" >"$bin_txt" 2>/dev/null; then
    echo "FAIL exec-c $rel"
    fail=$((fail + 1))
    continue
  fi
  run_n="$(normalize "$run_out")"
  bin_n="$(normalize "$bin_txt")"
  if [[ -z "$run_n" || -z "$bin_n" ]]; then
    echo "FAIL empty digest $rel run=[$run_n] c=[$bin_n]"
    fail=$((fail + 1))
  elif [[ "$run_n" != "$bin_n" ]]; then
    echo "FAIL parity $rel run=[$run_n] c=[$bin_n]"
    fail=$((fail + 1))
  else
    echo "OK parity $rel → $run_n"
  fi
done

if [[ "$fail" -ne 0 ]]; then
  echo "chs_semantic_parity: $fail failure(s)"
  exit 1
fi
echo "chs_semantic_parity: all OK"
exit 0
