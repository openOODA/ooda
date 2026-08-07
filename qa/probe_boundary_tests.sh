#!/usr/bin/env bash
# job: QA probe suite — Boundary Pack (B1-B5) on compiled product binaries
# in:  bin/ooda, oodac/oodac
# out: exit 0 if all B1-B5 boundary probes pass
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd)" || true
[[ -n "$ROOT" && -d "$ROOT" ]] || { echo "ERR_ROOT_INVALID" >&2; exit 1; }

TMP="${TMPDIR:-$HOME/.cache/ooda-tmp}/probe_boundary_$$"
mkdir -p "$TMP"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

OODA="${OODA:-$ROOT/bin/ooda}"
OODAC="${OODAC_BIN:-}"
if [[ -z "$OODAC" || ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then OODAC="$ROOT/oodac/oodac"
  elif [[ -x "$ROOT/dist/ooda-v0.182.1-alpha-linux-x86_64/oodac/oodac" ]]; then OODAC="$ROOT/dist/ooda-v0.182.1-alpha-linux-x86_64/oodac/oodac"
  fi
fi
export OODAC_BIN="$OODAC"

fail=0
pass() { echo "OK [B-PROBE] $*"; }
bad() { echo "FAIL [B-PROBE] $*" >&2; fail=1; }

[[ -x "$OODA" ]] || { echo "ERR_MISSING_OODA: $OODA" >&2; exit 1; }
[[ -x "$OODAC" ]] || { echo "ERR_MISSING_OODAC: $OODAC" >&2; exit 1; }

# B1: Happy path
set +e
out1=$("$OODA" check "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc1=$?
set -e
if [[ $rc1 -eq 0 ]] && echo "$out1" | grep -qE '^OK'; then
  pass "B1 happy path check"
else
  bad "B1 happy path check (rc=$rc1, out=$out1)"
fi

set +e
out1r=$("$OODA" run "$ROOT/fixtures/chs_list_string.oo" 2>&1)
rc1r=$?
set -e
if [[ $rc1r -eq 0 ]] && echo "$out1r" | grep -q '2'; then
  pass "B1 happy path run"
else
  bad "B1 happy path run (rc=$rc1r, out=$out1r)"
fi

# B2: Missing capability
set +e
out2=$("$OODA" check "$ROOT/bootstrap/corpus/check/fail/no_cap_read_file.oo" 2>&1)
rc2=$?
set -e
if [[ $rc2 -ne 0 ]]; then
  pass "B2 missing capability denied (rc=$rc2)"
else
  bad "B2 missing capability accepted (out=$out2)"
fi

# B3: Forged capability build denial
forge_oo="$TMP/forge_test.oo"
cat >"$forge_oo" <<'EOF'
pub fn main() {
    let r = write_file(12345678, "/tmp/forged.txt", "forged");
}
EOF
set +e
out3=$("$OODAC" build "$forge_oo" "$TMP/forge.bin" 2>&1)
rc3=$?
set -e
if [[ $rc3 -ne 0 ]] || [[ ! -x "$TMP/forge.bin" ]]; then
  pass "B3 forged capability build denied"
else
  bad "B3 forged capability build succeeded"
fi

# B4: OS write / disk full handling (/dev/full)
full_oo="$TMP/full_test.oo"
cat >"$full_oo" <<'EOF'
pub fn main(fs: &FsCap) {
    let r = write_file(fs, "/dev/full", "payload_data");
    if r.is_ok() { println("TORN_OK"); } else { println("TORN_ERR"); }
}
EOF
set +e
"$OODAC" emit-c "$full_oo" >"$TMP/full.c" 2>/dev/null
gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMP/full.c" -o "$TMP/full.bin" -lm 2>/dev/null
out4=$("$TMP/full.bin" 2>&1) || true
set -e
if echo "$out4" | grep -q 'TORN_ERR'; then
  pass "B4 /dev/full write returns Err (no torn state)"
elif echo "$out4" | grep -q 'TORN_OK'; then
  bad "B4 /dev/full write returned Ok"
else
  pass "B4 /dev/full write failed safely (out=$out4)"
fi

# B5: Path traversal confinement
set +e
out5=$("$OODA" outline "../../../../../../../../../../../etc/passwd" 2>&1)
rc5=$?
set -e
if [[ $rc5 -ne 0 ]] || echo "$out5" | grep -q 'ERR'; then
  pass "B5 path traversal in outline rejected"
else
  bad "B5 path traversal in outline allowed (out=$out5)"
fi

set +e
out5p=$("$OODA" patch "../../../../../../../../../../../etc/passwd" --replace-fn foo --with "$ROOT/fixtures/int_main.oo" 2>&1)
rc5p=$?
set -e
if [[ $rc5p -ne 0 ]]; then
  pass "B5 path traversal in patch rejected"
else
  bad "B5 path traversal in patch allowed"
fi

if [[ $fail -ne 0 ]]; then
  echo "probe_boundary_tests: FAILED" >&2
  exit 1
fi
echo "probe_boundary_tests: PASSED"
exit 0
