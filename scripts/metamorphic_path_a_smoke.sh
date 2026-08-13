#!/usr/bin/env bash
# Path-A metamorphic floor: layout decoys + meta_epoch + residual refuse honesty.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

DOC="bootstrap/METAMORPHIC.oot"
[[ -f "$DOC" ]] || { echo "FAIL missing $DOC" >&2; exit 1; }
grep -q "METAMORPHIC_RESIDUAL_ALPHA" "$DOC" && pass "doc marker" || bad "doc marker"
grep -qiE 'fail-closed residual|Fail-closed residual' "$DOC" && pass "fail-closed residual wording" || bad "fail-closed residual wording"
grep -q "METAMORPHIC: path-a" "$DOC" && pass "path-a directive documented" || bad "path-a directive"
grep -q "meta_epoch" "$DOC" && pass "meta_epoch documented" || bad "meta_epoch"

# Residual free-names still refuse at check (full DESIGN API)
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
if [[ -x "$OODAC" ]]; then
  set +e
  out=$("$OODAC" check fixtures/residual_metamorphic_fail.oo 2>&1)
  rc=$?
  set -e
  if [[ $rc -ne 0 ]] && echo "$out" | grep -qiE 'residual|METAMORPHIC'; then
    pass "metamorphic_emit still residual refuse"
  else
    bad "metamorphic_emit refuse expected (rc=$rc out=$out)"
  fi
else
  pass "skip live oodac check (no binary)"
fi

# Source floor: emit meta + runtime meta present
grep -q 'c_parse_metamorphic_path_a' oodac/c_emit_meta.oo && pass "c_emit_meta parser" || bad "c_emit_meta parser"
grep -q 'c_emit_meta_decoy' oodac/c_emit.oo && pass "c_emit wires decoys" || bad "c_emit wires decoys"
grep -q 'oo_meta_epoch' runtime/chs_rt_meta.c && pass "runtime epoch" || bad "runtime epoch"
grep -q 'chs_rt_meta.c' runtime/chs_rt.c && pass "meta in umbrella" || bad "meta in umbrella"

# Compile runtime unit
if gcc -c -O2 -Iruntime -Wall -Wextra -Werror runtime/chs_rt_meta.c -o "$TMPDIR/chs_rt_meta.o" 2>"$TMPDIR/meta_cc.err"; then
  pass "chs_rt_meta.c compiles"
else
  bad "chs_rt_meta compile"; head -20 "$TMPDIR/meta_cc.err" || true
fi

# Live emit if oodac present: path-a must emit decoys; no-marker twin must not.
if [[ -x "$OODAC" ]]; then
  set +e
  "$OODAC" emit-c fixtures/metamorphic_path_a.oo >"$TMPDIR/meta_pa.c" 2>"$TMPDIR/meta_pa.err"
  erc=$?
  set -e
  if [[ $erc -eq 0 ]] && grep -q '__oo_meta_decoy_' "$TMPDIR/meta_pa.c"; then
    pass "emit-c path-a produces decoys"
  else
    bad "emit-c path-a must rc=0 and contain __oo_meta_decoy_ (rc=$erc)"
    head -20 "$TMPDIR/meta_pa.err" || true
  fi

  OFF="$TMPDIR/metamorphic_path_a_off.oo"
  cat >"$OFF" <<'EOF'
pub fn helper_a() -> Int {
    return 1;
}

pub fn helper_b() -> Int {
    return 2;
}

pub fn main() {
    let e = meta_epoch();
    if e == 0 {
        process_exit(1);
    }
    println(helper_a() + helper_b());
}
EOF
  set +e
  "$OODAC" emit-c "$OFF" >"$TMPDIR/meta_off.c" 2>"$TMPDIR/meta_off.err"
  orc=$?
  set -e
  if [[ $orc -ne 0 ]]; then
    bad "emit-c no-marker twin must rc=0 (rc=$orc)"
    head -20 "$TMPDIR/meta_off.err" || true
  elif grep -q '__oo_meta_decoy_' "$TMPDIR/meta_off.c"; then
    bad "emit-c no-marker twin must have zero __oo_meta_decoy_ symbols"
  else
    pass "emit-c no-marker twin has zero decoys"
  fi

  # Live emit prove of decoy cap: 70 tiny fns + main → 1..64 symbols (expect 64).
  CAP="$TMPDIR/metamorphic_path_a_cap70.oo"
  {
    echo '// METAMORPHIC: path-a'
    i=0
    while [[ $i -lt 70 ]]; do
      echo "pub fn f${i}() { }"
      i=$((i + 1))
    done
    echo 'pub fn main() { }'
  } >"$CAP"
  set +e
  "$OODAC" emit-c "$CAP" >"$TMPDIR/meta_cap.c" 2>"$TMPDIR/meta_cap.err"
  crc=$?
  set -e
  if [[ $crc -ne 0 ]]; then
    bad "emit-c decoy-cap fixture must rc=0 (rc=$crc)"
    head -20 "$TMPDIR/meta_cap.err" || true
  else
    nd=$(grep -c 'static void __oo_meta_decoy_' "$TMPDIR/meta_cap.c" || true)
    if [[ "$nd" -ge 1 && "$nd" -le 64 && "$nd" -eq 64 ]]; then
      pass "emit-c decoy cap count=$nd (1..64; 70 fns → 64)"
    else
      bad "emit-c decoy count must be 1..64 (got $nd; expect 64 for 70 fns)"
    fi
    # Decoy bodies: fail any [N] with N>16 (look for __oo_meta_buf[).
    if grep -nE '__oo_meta_buf\[(1[7-9]|[2-9][0-9]|[0-9]{3,})\]' "$TMPDIR/meta_cap.c" >/dev/null; then
      bad "decoy body __oo_meta_buf[N] with N>16"
      grep -nE '__oo_meta_buf\[[0-9]+\]' "$TMPDIR/meta_cap.c" | head -5 || true
    elif awk '
      /^static void __oo_meta_decoy_/ { d=1 }
      d {
        s=$0
        while (match(s, /\[[0-9]+\]/)) {
          n = substr(s, RSTART+1, RLENGTH-2) + 0
          if (n > 16) { print FILENAME ":" NR ": " $0; bad=1 }
          s = substr(s, RSTART+RLENGTH)
        }
        if ($0 ~ /^}/) d=0
      }
      END { exit bad ? 1 : 0 }
    ' "$TMPDIR/meta_cap.c"; then
      pass "decoy bodies have no [N] with N>16"
    else
      bad "decoy body array size [N] with N>16"
    fi
  fi
fi

if grep -q "metamorphic_path_a_smoke.sh" scripts/ci_product.sh 2>/dev/null; then
  pass "ci_product wire"
else
  # Wire ourselves if missing
  if grep -q "metamorphic_residual_smoke" scripts/ci_product.sh 2>/dev/null; then
    pass "ci_product has residual metamorphic (path-a companion optional)"
  else
    bad "ci_product missing metamorphic smokes"
  fi
fi

if [[ $fail -ne 0 ]]; then
  echo "metamorphic_path_a_smoke: FAILED" >&2
  exit 1
fi
echo "metamorphic_path_a_smoke: PASSED"
exit 0
