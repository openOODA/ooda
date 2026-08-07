# job: caps matrix rails — check deny/allow + real Fs/Sys/Env emit+run; net residual
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
OODA="${OODA:-$ROOT/bin/ooda}"
if [[ ! -x "$OODAC" ]]; then
  echo "ERR_NO_OODAC: need $OODAC" >&2
  exit 1
fi
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
CHECK_PASS="$ROOT/bootstrap/corpus/check/pass"
CHECK_FAIL="$ROOT/bootstrap/corpus/check/fail"
for f in "$CHECK_FAIL"/*.oo; do
  [[ -f "$f" ]] || continue
  base="$(basename "$f")"
  set +e
  "$OODAC" check "$f" >"$TMPDIR/cm_${base}.out" 2>"$TMPDIR/cm_${base}.err"
  rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    bad "check accepted deny fixture $base"
  elif ! grep -qE 'capability|ERR' "$TMPDIR/cm_${base}.out" "$TMPDIR/cm_${base}.err" 2>/dev/null; then
    bad "check fail $base missing ERR (exit=$rc)"
  else
    pass "check deny $base"
  fi
done
for f in "$CHECK_PASS"/*.oo; do
  [[ -f "$f" ]] || continue
  base="$(basename "$f")"
  set +e
  "$OODAC" check "$f" >"$TMPDIR/cm_p_${base}.out" 2>"$TMPDIR/cm_p_${base}.err"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]] || ! grep -qE '^OK' "$TMPDIR/cm_p_${base}.out"; then
    bad "check rejected pass fixture $base exit=$rc"
    cat "$TMPDIR/cm_p_${base}.out" "$TMPDIR/cm_p_${base}.err" | head -5 || true
  else
    pass "check allow $base"
  fi
done
if [[ -x "$OODA" ]]; then
  set +e
  "$OODA" check "$CHECK_FAIL/no_cap_fetch.oo" >"$TMPDIR/cm_prod.out" 2>"$TMPDIR/cm_prod.err"
  prc=$?
  set -e
  if [[ $prc -eq 0 ]]; then bad "product check accepted no_cap_fetch"; else pass "product check deny no_cap_fetch"; fi
fi
NET_SRC="$TMPDIR/cm_net_cap.oo"
cat >"$NET_SRC" <<'EOF'
pub fn main(net: &NetCap) {
    let r = fetch("http://example.invalid");
}
EOF
set +e
"$OODAC" emit-c "$NET_SRC" >"$TMPDIR/cm_net.c" 2>"$TMPDIR/cm_net.err"
nrc=$?
set -e
if grep -qE $'^ERR\tc_emit\tnet residual' "$TMPDIR/cm_net.c" "$TMPDIR/cm_net.err" 2>/dev/null; then
  pass "emit net residual fail-closed"
elif [[ $nrc -ne 0 ]]; then
  pass "emit net residual non-zero exit"
else
  bad "emit lowered/accepted net fetch (must residual)"
fi
FS_SRC="$TMPDIR/cm_fs.oo"
FS_PATH="$TMPDIR/cm_fs_round.txt"
cat >"$FS_SRC" <<EOF
pub fn main(fs: &FsCap) {
    let w = write_file(fs, "$FS_PATH", "caps-matrix-ok");
    if w.is_ok() {
        let r = read_file(fs, "$FS_PATH");
        if r.is_ok() {
            println("fs-ok");
        }
    }
}
EOF
set +e
"$OODAC" emit-c "$FS_SRC" >"$TMPDIR/cm_fs.c" 2>"$TMPDIR/cm_fs.err"
frc=$?
set -e
if [[ $frc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_fs.c"; then
  bad "emit fs roundtrip"
  cat "$TMPDIR/cm_fs.err" | head -10 || true
else
  if ! grep -q 'oo_write_file' "$TMPDIR/cm_fs.c" || ! grep -q 'oo_read_file' "$TMPDIR/cm_fs.c"; then
    bad "emit fs missing oo_read/write"
  elif ! grep -qE 'oo_write_file\(fs,' "$TMPDIR/cm_fs.c"; then
    bad "emit write_file must pass cap as first arg (runtime seal)"
  elif ! grep -qE 'oo_read_file\(fs,' "$TMPDIR/cm_fs.c"; then
    bad "emit read_file must pass cap as first arg (runtime seal)"
  else
    gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMPDIR/cm_fs.c" -o "$TMPDIR/cm_fs.bin" -lm
    rc_bin=0; out=$("$TMPDIR/cm_fs.bin" 2>&1) || rc_bin=$?
    if echo "$out" | grep -q 'fs-ok'; then
      pass "runtime Fs write+read"
    else
      bad "runtime Fs roundtrip out=$out"
    fi
    sed 's/long long fs = OO_CAP_FS/long long fs = 0LL/' "$TMPDIR/cm_fs.c" >"$TMPDIR/cm_fs_forge.c"
    gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMPDIR/cm_fs_forge.c" -o "$TMPDIR/cm_fs_forge.bin" -lm
    set +e
    forge_out=$("$TMPDIR/cm_fs_forge.bin" 2>&1)
    forge_rc=$?
    set -e
    if [[ $forge_rc -ne 0 ]] && echo "$forge_out" | grep -qE $'ERR[\t ]*cap'; then
      pass "runtime Fs forged cap deny"
    else
      bad "runtime forge cap should deny out=$forge_out rc=$forge_rc"
    fi
  fi
fi

# --- real Env env_get ---
ENV_SRC="$TMPDIR/cm_env.oo"
cat >"$ENV_SRC" <<'EOF'
pub fn main(env: &EnvCap) {
    let r = env_get(env, "PATH");
    if r.is_ok() {
        println("env-ok");
    }
}
EOF
set +e
"$OODAC" emit-c "$ENV_SRC" >"$TMPDIR/cm_env.c" 2>"$TMPDIR/cm_env.err"
erc=$?
set -e
if [[ $erc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_env.c"; then
  bad "emit env_get"
else
  if ! grep -q 'oo_env_get' "$TMPDIR/cm_env.c"; then
    bad "emit env_get not lowered to oo_env_get"
  elif ! grep -qE 'oo_env_get\(env,' "$TMPDIR/cm_env.c"; then
    bad "emit env_get must pass cap as first arg (runtime seal)"
  else
    gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMPDIR/cm_env.c" -o "$TMPDIR/cm_env.bin" -lm
    rc_bin=0; out=$("$TMPDIR/cm_env.bin" 2>&1) || rc_bin=$?
    if echo "$out" | grep -q 'env-ok'; then
      pass "runtime Env env_get"
    else
      bad "runtime env_get out=$out"
    fi
  fi
fi

# --- real Sys sys_exec ---
SYS_SRC="$TMPDIR/cm_sys.oo"
cat >"$SYS_SRC" <<'EOF'
pub fn main(sys: &SysCap) {
    let r = sys_exec(sys, "sh", "-c", "true");
    if r.is_ok() {
        println("sys-ok");
    }
}
EOF
set +e
"$OODAC" emit-c "$SYS_SRC" >"$TMPDIR/cm_sys.c" 2>"$TMPDIR/cm_sys.err"
src_rc=$?
set -e
if [[ $src_rc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_sys.c"; then
  bad "emit sys_exec"
else
  if ! grep -q 'oo_sys_exec1' "$TMPDIR/cm_sys.c"; then
    bad "emit sys_exec not lowered"
  else
    gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMPDIR/cm_sys.c" -o "$TMPDIR/cm_sys.bin" -lm
    rc_bin=0; out=$("$TMPDIR/cm_sys.bin" 2>&1) || rc_bin=$?
    if echo "$out" | grep -q 'sys-ok'; then
      pass "runtime Sys sys_exec"
    else
      bad "runtime sys_exec out=$out"
    fi
  fi
fi

# --- path_exists lower ---
PE_SRC="$TMPDIR/cm_pe.oo"
cat >"$PE_SRC" <<'EOF'
pub fn main(fs: &FsCap) {
    if path_exists(fs, "/tmp") {
        println("pe-ok");
    }
}
EOF
set +e
"$OODAC" emit-c "$PE_SRC" >"$TMPDIR/cm_pe.c" 2>"$TMPDIR/cm_pe.err"
perc=$?
set -e
if [[ $perc -ne 0 ]] || ! grep -q 'oo_path_exists' "$TMPDIR/cm_pe.c"; then
  bad "emit path_exists"
else
  gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMPDIR/cm_pe.c" -o "$TMPDIR/cm_pe.bin" -lm
  rc_bin=0; out=$("$TMPDIR/cm_pe.bin" 2>&1) || rc_bin=$?
  if echo "$out" | grep -q 'pe-ok'; then
    pass "runtime Fs path_exists"
  else
    bad "runtime path_exists out=$out"
  fi
fi

# --- C1: magic-int forge must not build ---
FORGE="$TMPDIR/cm_forge.oo"
cat >"$FORGE" <<'EOF'
pub fn main() {
    let r = write_file(1330595411, "/tmp/ooda_cm_forge.txt", "FORGED");
    if r.is_ok() { println("FORGE_OK"); }
}
EOF
set +e
"$OODAC" build "$FORGE" "$TMPDIR/cm_forge.bin" >"$TMPDIR/cm_forge.out" 2>"$TMPDIR/cm_forge.err"
frc=$?
set -e
if [[ $frc -eq 0 ]] || [[ -x "$TMPDIR/cm_forge.bin" ]]; then
  bad "magic-int forge build should fail-closed"
else
  pass "magic-int forge build denied"
fi

# --- C2: /dev/full must not torn-success ---
FULL="$TMPDIR/cm_full.oo"
cat >"$FULL" <<'EOF'
pub fn main(fs: &FsCap) {
    let r = write_file(fs, "/dev/full", "payload");
    if r.is_ok() { println("TORN_OK"); } else { println("TORN_ERR"); }
}
EOF
set +e
"$OODAC" emit-c "$FULL" >"$TMPDIR/cm_full.c" 2>/dev/null
gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMPDIR/cm_full.c" -o "$TMPDIR/cm_full.bin" -lm 2>/dev/null
rc_full=0; fout=$("$TMPDIR/cm_full.bin" 2>/dev/null) || rc_full=$?
set -e
if echo "$fout" | grep -q 'TORN_ERR'; then
  pass "write /dev/full is Err (no torn Ok)"
elif echo "$fout" | grep -q 'TORN_OK'; then
  bad "write /dev/full torn success"
else
  pass "write /dev/full non-Ok (out=$fout)"
fi

if [[ $fail -ne 0 ]]; then
  echo "caps_matrix_smoke: FAILED" >&2
  exit 1
fi
echo "caps_matrix_smoke: PASSED"
exit 0
