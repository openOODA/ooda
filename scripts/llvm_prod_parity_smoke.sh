#!/usr/bin/env bash
# job: M119 production LLVM parity — CHS four + multi-module import at O0/O3
# drives real oodac emit-llvm + llvm_link (not mocked)
# fail-closed: missing tools / emit/link/run mismatch → non-zero
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
LINK="$ROOT/scripts/llvm_link.sh"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
TMP="$TMPDIR/llvm_prod_parity_$$"
mkdir -p "$TMP"
trap 'rm -rf "$TMP"' EXIT

[[ -x "$OODAC" ]] || { echo "ERR_NO_COMPILER: $OODAC" >&2; exit 1; }
[[ -f "$LINK" ]] || { echo "ERR_NO_LLVM_LINK: $LINK" >&2; exit 1; }
[[ -f "$ROOT/runtime/chs_rt.c" ]] || { echo "ERR_NO_RUNTIME" >&2; exit 1; }

# name|relpath|build_mode (single=emit-llvm import-expand, pure=Backend-C pure multi for golden)
FIXTURES=(
  "chs_hello|fixtures/chs_hello.oo|single"
  "while_count|fixtures/while_count.oo|single"
  "for_range|fixtures/for_range.oo|single"
  "chs_list_string|fixtures/chs_list_string.oo|single"
  "m119_multi|fixtures/m119_multi/main.oo|import"
)

golden_c() {
  local src="$1" bin="$2"
  # pure multi handles imports
  PURE_SKIP_CHECK=1 OODAC_BIN="$OODAC" bash "$ROOT/scripts/my_pure_build.sh" "$src" "$bin" \
    >"$TMP/c_build.out" 2>"$TMP/c_build.err" || {
    echo "FAIL Backend-C build $src" >&2
    cat "$TMP/c_build.err" >&2 || true
    exit 1
  }
  timeout 5 "$bin" | tr -d '\r' | sed -e 's/[[:space:]]*$//'
}

run_llvm() {
  local src="$1" ll="$2" bin="$3" opt="$4"
  timeout 60 "$OODAC" emit-llvm "$src" >"$ll" 2>"$TMP/ll_emit.err" || {
    echo "FAIL emit-llvm $src" >&2
    cat "$TMP/ll_emit.err" >&2 || true
    exit 1
  }
  [[ -s "$ll" ]] && grep -q 'define ' "$ll" || {
    echo "FAIL empty/missing define: $src" >&2
    exit 1
  }
  bash "$LINK" "$opt" "$ll" "$bin" >/dev/null
  timeout 5 "$bin" 2>/dev/null | tr -d '\r' | sed -e 's/[[:space:]]*$//'
}

fail=0
for entry in "${FIXTURES[@]}"; do
  name="${entry%%|*}"
  rest="${entry#*|}"
  rel="${rest%%|*}"
  src="$ROOT/$rel"
  [[ -f "$src" ]] || { echo "FAIL missing $src" >&2; fail=1; continue; }
  echo "== $name Backend-C golden =="
  gold="$(golden_c "$src" "$TMP/${name}_c.bin")"
  printf '%s\n' "$gold" | sed 's/^/  C: /'
  for opt in -O0 -O3; do
    echo "== $name LLVM $opt =="
    got="$(run_llvm "$src" "$TMP/${name}${opt}.ll" "$TMP/${name}${opt}.bin" "$opt")"
    printf '%s\n' "$got" | sed "s/^/  LLVM $opt: /"
    if [[ "$got" != "$gold" ]]; then
      echo "FAIL parity $name $opt" >&2
      echo "  expected: $(printf '%q' "$gold")" >&2
      echo "  got:      $(printf '%q' "$got")" >&2
      fail=1
    else
      echo "OK parity $name $opt"
    fi
  done
done

if [[ "$fail" -ne 0 ]]; then
  echo "llvm_prod_parity_smoke: FAILED" >&2
  exit 1
fi
echo "llvm_prod_parity_smoke: PASSED (CHS×4 + multi-module import × O0/O3)"
exit 0
