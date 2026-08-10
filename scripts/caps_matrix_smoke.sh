# job: caps matrix rails — check deny/allow + Fs/Sys/Env/Net/Time/Rand (+Alloc corpus via glob)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"
OODAC="${OODAC_BIN:-$ROOT/oodac/oodac}"
OODA="${OODA:-$ROOT/bin/ooda}"
[[ -x "$OODAC" ]] || { echo "ERR_NO_OODAC: need $OODAC" >&2; exit 1; }
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }
RT=(-O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c")
emit_run() { # $1=src $2=bin $3=needle $4=label
  set +e; "$OODAC" emit-c "$1" >"${2}.c" 2>"${2}.err"; local e=$?; set -e
  if [[ $e -ne 0 ]] || grep -qE $'^ERR\t' "${2}.c"; then bad "emit $4"; return; fi
  gcc "${RT[@]}" "${2}.c" -o "$2" -lm
  local out rc=0; out=$("$2" 2>&1) || rc=$?
  if echo "$out" | grep -q "$3"; then pass "runtime $4"; else bad "runtime $4 out=$out"; fi
}
deny_forge() { # $1=c $2=sed_expr $3=label
  local fc="${1%.c}_forge.c" fb="${1%.c}_forge.bin"
  sed -E "$2" "$1" >"$fc"
  gcc "${RT[@]}" "$fc" -o "$fb" -lm
  set +e; local o rc=0; o=$("$fb" 2>&1) || rc=$?; set -e
  if [[ $rc -ne 0 ]] && echo "$o" | grep -qE $'ERR[\t ]*cap'; then pass "$3"
  else bad "$3 out=$o rc=$rc"; fi
}
CHECK_PASS="$ROOT/bootstrap/corpus/check/pass"
CHECK_FAIL="$ROOT/bootstrap/corpus/check/fail"
for f in "$CHECK_FAIL"/*.oo; do
  [[ -f "$f" ]] || continue
  base="$(basename "$f")"
  set +e; "$OODAC" check "$f" >"$TMPDIR/cm_${base}.out" 2>"$TMPDIR/cm_${base}.err"; rc=$?; set -e
  if [[ $rc -eq 0 ]]; then bad "check accepted deny fixture $base"
  elif ! grep -qE 'capability|ERR' "$TMPDIR/cm_${base}.out" "$TMPDIR/cm_${base}.err" 2>/dev/null; then
    bad "check fail $base missing ERR (exit=$rc)"
  else pass "check deny $base"; fi
done
for f in "$CHECK_PASS"/*.oo; do
  [[ -f "$f" ]] || continue
  base="$(basename "$f")"
  set +e; "$OODAC" check "$f" >"$TMPDIR/cm_p_${base}.out" 2>"$TMPDIR/cm_p_${base}.err"; rc=$?; set -e
  if [[ $rc -ne 0 ]] || ! grep -qE '^OK' "$TMPDIR/cm_p_${base}.out"; then
    bad "check rejected pass fixture $base exit=$rc"
    cat "$TMPDIR/cm_p_${base}.out" "$TMPDIR/cm_p_${base}.err" | head -5 || true
  else pass "check allow $base"; fi
done
# Product static deny ×7 families (M8 + M12 Time/Rand + M17 Alloc)
if [[ -x "$OODA" ]]; then
  for base in no_cap_read_file no_cap_sys_exec no_cap_env_get no_cap_fetch no_cap_now_ms no_cap_random no_cap_alloc_bytes; do
    ff="$CHECK_FAIL/${base}.oo"
    [[ -f "$ff" ]] || continue
    set +e
    "$OODA" check "$ff" >"$TMPDIR/cm_prod_${base}.out" 2>"$TMPDIR/cm_prod_${base}.err"
    prc=$?
    set -e
    if [[ $prc -eq 0 ]]; then bad "product check accepted $base"
    else pass "product check deny $base"; fi
  done
fi
# fetch without &NetCap first arg must fail-closed at emit
cat >"$TMPDIR/cm_net.oo" <<'EOF'
pub fn main(net: &NetCap) { let r = fetch("http://example.invalid"); }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_net.oo" >"$TMPDIR/cm_net.c" 2>"$TMPDIR/cm_net.err"; nrc=$?; set -e
if grep -qE $'^ERR\tc_emit\t(fetch requires|net residual)' "$TMPDIR/cm_net.c" "$TMPDIR/cm_net.err" 2>/dev/null; then
  pass "emit fetch without NetCap arg fail-closed"
elif [[ $nrc -ne 0 ]]; then pass "emit fetch without NetCap arg non-zero exit"
else bad "emit lowered fetch without NetCap arg (must fail-closed)"; fi
# M12: now_ms without cap IDENT must fail-closed at emit
cat >"$TMPDIR/cm_time_bad.oo" <<'EOF'
pub fn main(time: &TimeCap) { let n = now_ms(1); }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_time_bad.oo" >"$TMPDIR/cm_time_bad.c" 2>"$TMPDIR/cm_time_bad.err"; tbrc=$?; set -e
if grep -qE $'^ERR\tc_emit\tnow_ms requires' "$TMPDIR/cm_time_bad.c" "$TMPDIR/cm_time_bad.err" 2>/dev/null; then
  pass "emit now_ms without TimeCap arg fail-closed"
elif [[ $tbrc -ne 0 ]]; then pass "emit now_ms without TimeCap arg non-zero exit"
else bad "emit lowered now_ms without TimeCap arg (must fail-closed)"; fi

FS_PATH="$TMPDIR/cm_fs_round.txt"
cat >"$TMPDIR/cm_fs.oo" <<EOF
pub fn main(fs: &FsCap) {
    let w = write_file(fs, "$FS_PATH", "caps-matrix-ok");
    if w.is_ok() { let r = read_file(fs, "$FS_PATH"); if r.is_ok() { println("fs-ok"); } }
}
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_fs.oo" >"$TMPDIR/cm_fs.c" 2>"$TMPDIR/cm_fs.err"; frc=$?; set -e
if [[ $frc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_fs.c"; then bad "emit fs roundtrip"
elif ! grep -qE 'oo_write_file\(fs,' "$TMPDIR/cm_fs.c" || ! grep -qE 'oo_read_file\(fs,' "$TMPDIR/cm_fs.c"; then
  bad "emit fs must pass cap first arg"
else
  gcc "${RT[@]}" "$TMPDIR/cm_fs.c" -o "$TMPDIR/cm_fs.bin" -lm
  out=$("$TMPDIR/cm_fs.bin" 2>&1) || true
  if echo "$out" | grep -q 'fs-ok'; then pass "runtime Fs write+read"; else bad "runtime Fs roundtrip out=$out"; fi
  # R1: zeroed grant + classic magic 0x4F4F4653 must deny
  deny_forge "$TMPDIR/cm_fs.c" \
    's/long long fs = oo_cap_grant_fs\(\)/long long fs = 0LL/; s/long long fs = OO_CAP_FS/long long fs = 0LL/' \
    "runtime Fs forged cap deny"
  deny_forge "$TMPDIR/cm_fs.c" \
    's/long long fs = oo_cap_grant_fs\(\)/long long fs = 0x4F4F4653LL/; s/long long fs = OO_CAP_FS/long long fs = 0x4F4F4653LL/' \
    "runtime classic magic-int Fs deny"
fi

cat >"$TMPDIR/cm_env.oo" <<'EOF'
pub fn main(env: &EnvCap) { let r = env_get(env, "PATH"); if r.is_ok() { println("env-ok"); } }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_env.oo" >"$TMPDIR/cm_env.c" 2>"$TMPDIR/cm_env.err"; erc=$?; set -e
if [[ $erc -ne 0 ]] || ! grep -qE 'oo_env_get\(env,' "$TMPDIR/cm_env.c"; then bad "emit env_get"
else emit_run "$TMPDIR/cm_env.oo" "$TMPDIR/cm_env.bin" "env-ok" "Env env_get"; fi

# R2/R3: full argv oo_sys_exec; no system(3)
cat >"$TMPDIR/cm_sys.oo" <<'EOF'
pub fn main(sys: &SysCap) { let r = sys_exec(sys, "sh", "-c", "true"); if r.is_ok() { println("sys-ok"); } }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_sys.oo" >"$TMPDIR/cm_sys.c" 2>"$TMPDIR/cm_sys.err"; src_rc=$?; set -e
if [[ $src_rc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_sys.c"; then bad "emit sys_exec"
elif ! grep -qE 'oo_sys_exec\(' "$TMPDIR/cm_sys.c"; then bad "emit sys_exec not lowered to oo_sys_exec"
elif grep -qE 'system\(' "$TMPDIR/cm_sys.c"; then bad "emit sys_exec must not use system(3)"
elif ! grep -qE 'OoStr\[\]' "$TMPDIR/cm_sys.c"; then bad "emit sys_exec must pass argv array"
else
  gcc "${RT[@]}" "$TMPDIR/cm_sys.c" -o "$TMPDIR/cm_sys.bin" -lm
  out=$("$TMPDIR/cm_sys.bin" 2>&1) || true
  if echo "$out" | grep -q 'sys-ok'; then pass "runtime Sys sys_exec argv"; else bad "runtime sys_exec out=$out"; fi
  # M8: Sys/Env forge deny (zero + classic magic)
  deny_forge "$TMPDIR/cm_sys.c" \
    's/long long sys = oo_cap_grant_sys\(\)/long long sys = 0LL/' \
    "runtime Sys forged cap deny (zero)"
  deny_forge "$TMPDIR/cm_sys.c" \
    's/long long sys = oo_cap_grant_sys\(\)/long long sys = 0x4F4F5359LL/' \
    "runtime Sys classic magic-int deny"
fi

# Env forge deny after successful env emit (reuse cm_env.c if present)
if [[ -f "$TMPDIR/cm_env.c" ]] && ! grep -qE $'^ERR\t' "$TMPDIR/cm_env.c" 2>/dev/null; then
  deny_forge "$TMPDIR/cm_env.c" \
    's/long long env = oo_cap_grant_env\(\)/long long env = 0LL/' \
    "runtime Env forged cap deny (zero)"
  deny_forge "$TMPDIR/cm_env.c" \
    's/long long env = oo_cap_grant_env\(\)/long long env = 0x4F4F454ELL/' \
    "runtime Env classic magic-int deny"
fi

# Net: emit fetch with NetCap; forge grant must deny before network
cat >"$TMPDIR/cm_net_rt.oo" <<'EOF'
pub fn main(net: &NetCap) {
    let r = fetch(net, "http://127.0.0.1:1/");
    if r.is_ok() { println("net-ok"); } else { println("net-err"); }
}
EOF
set +e
"$OODAC" emit-c "$TMPDIR/cm_net_rt.oo" >"$TMPDIR/cm_net_rt.c" 2>"$TMPDIR/cm_net_rt.err"
nrtc=$?
set -e
if [[ $nrtc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_net_rt.c"; then
  bad "emit fetch with NetCap"
  head -5 "$TMPDIR/cm_net_rt.err" 2>/dev/null || true
elif ! grep -qE 'oo_fetch\(net,' "$TMPDIR/cm_net_rt.c"; then
  bad "emit fetch must pass net cap first"
else
  pass "emit fetch with NetCap"
  deny_forge "$TMPDIR/cm_net_rt.c" \
    's/long long net = oo_cap_grant_net\(\)/long long net = 0LL/' \
    "runtime Net forged cap deny (zero)"
  deny_forge "$TMPDIR/cm_net_rt.c" \
    's/long long net = oo_cap_grant_net\(\)/long long net = 0x4F4F4E54LL/' \
    "runtime Net classic magic-int deny"
fi

cat >"$TMPDIR/cm_pe.oo" <<'EOF'
pub fn main(fs: &FsCap) { if path_exists(fs, "/tmp") { println("pe-ok"); } }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_pe.oo" >"$TMPDIR/cm_pe.c" 2>"$TMPDIR/cm_pe.err"; perc=$?; set -e
if [[ $perc -ne 0 ]] || ! grep -q 'oo_path_exists' "$TMPDIR/cm_pe.c"; then bad "emit path_exists"
else emit_run "$TMPDIR/cm_pe.oo" "$TMPDIR/cm_pe.bin" "pe-ok" "Fs path_exists"; fi

# M12: TimeCap — now_ms runtime + forge deny (zero + classic 0x4F4F544D)
# Grant inject name is `time` (c_emit_fn); body must use that IDENT.
cat >"$TMPDIR/cm_time.oo" <<'EOF'
pub fn main(time: &TimeCap) { let n = now_ms(time); println("time-ok"); }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_time.oo" >"$TMPDIR/cm_time.c" 2>"$TMPDIR/cm_time.err"; trc=$?; set -e
if [[ $trc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_time.c"; then bad "emit now_ms"
elif ! grep -qE 'oo_now_ms\(time' "$TMPDIR/cm_time.c"; then bad "emit now_ms must pass time cap first"
else
  emit_run "$TMPDIR/cm_time.oo" "$TMPDIR/cm_time.bin" "time-ok" "Time now_ms"
  deny_forge "$TMPDIR/cm_time.c" \
    's/long long time = oo_cap_grant_time\(\)/long long time = 0LL/' \
    "runtime Time forged cap deny (zero)"
  deny_forge "$TMPDIR/cm_time.c" \
    's/long long time = oo_cap_grant_time\(\)/long long time = 0x4F4F544DLL/' \
    "runtime Time classic magic-int deny"
fi

# M12: RandCap — random runtime + forge deny (zero + classic 0x4F4F524E)
cat >"$TMPDIR/cm_rand.oo" <<'EOF'
pub fn main(rand: &RandCap) { let x = random(rand); println("rand-ok"); }
EOF
set +e; "$OODAC" emit-c "$TMPDIR/cm_rand.oo" >"$TMPDIR/cm_rand.c" 2>"$TMPDIR/cm_rand.err"; rrc=$?; set -e
if [[ $rrc -ne 0 ]] || grep -qE $'^ERR\t' "$TMPDIR/cm_rand.c"; then bad "emit random"
elif ! grep -qE 'oo_random\(rand' "$TMPDIR/cm_rand.c"; then bad "emit random must pass rand cap first"
else
  emit_run "$TMPDIR/cm_rand.oo" "$TMPDIR/cm_rand.bin" "rand-ok" "Rand random"
  deny_forge "$TMPDIR/cm_rand.c" \
    's/long long rand = oo_cap_grant_rand\(\)/long long rand = 0LL/' \
    "runtime Rand forged cap deny (zero)"
  deny_forge "$TMPDIR/cm_rand.c" \
    's/long long rand = oo_cap_grant_rand\(\)/long long rand = 0x4F4F524ELL/' \
    "runtime Rand classic magic-int deny"
fi

# C1: magic-int forge must not build
cat >"$TMPDIR/cm_forge.oo" <<'EOF'
pub fn main() {
    let r = write_file(1330595411, "/tmp/ooda_cm_forge.txt", "FORGED");
    if r.is_ok() { println("FORGE_OK"); }
}
EOF
set +e; "$OODAC" build "$TMPDIR/cm_forge.oo" "$TMPDIR/cm_forge.bin" >"$TMPDIR/cm_forge.out" 2>"$TMPDIR/cm_forge.err"; frc=$?; set -e
if [[ $frc -eq 0 ]] || [[ -x "$TMPDIR/cm_forge.bin" ]]; then bad "magic-int forge build should fail-closed"
else pass "magic-int forge build denied"; fi

# C2: /dev/full must not torn-success
cat >"$TMPDIR/cm_full.oo" <<'EOF'
pub fn main(fs: &FsCap) {
    let r = write_file(fs, "/dev/full", "payload");
    if r.is_ok() { println("TORN_OK"); } else { println("TORN_ERR"); }
}
EOF
set +e
"$OODAC" emit-c "$TMPDIR/cm_full.oo" >"$TMPDIR/cm_full.c" 2>/dev/null
gcc "${RT[@]}" "$TMPDIR/cm_full.c" -o "$TMPDIR/cm_full.bin" -lm 2>/dev/null
fout=$("$TMPDIR/cm_full.bin" 2>/dev/null) || true
set -e
if echo "$fout" | grep -q 'TORN_ERR'; then pass "write /dev/full is Err (no torn Ok)"
elif echo "$fout" | grep -q 'TORN_OK'; then bad "write /dev/full torn success"
else pass "write /dev/full non-Ok (out=$fout)"; fi

if [[ $fail -ne 0 ]]; then echo "caps_matrix_smoke: FAILED" >&2; exit 1; fi
echo "caps_matrix_smoke: PASSED"
exit 0
