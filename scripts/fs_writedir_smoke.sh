#!/usr/bin/env bash
# HIGH 6.4 — OODA_FS_WRITEDIR write boundary (path A)
# Path A: unset/empty → deny; root "/" → deny; allow create+overwrite under dir;
#         deny outside and .. escape; leaf symlink outside → deny.
# Residual: parent dentry rename TOCTOU (no landlock); leaf symlink inside
#           allowdir fails open (O_NOFOLLOW fail-closed); no read allowdir.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export TMPDIR="${TMPDIR:-.ooda-cache/ooda-tmp}"
mkdir -p "$TMPDIR"
fail=0
pass() { echo "OK $*"; }
bad() { echo "FAIL $*" >&2; fail=1; }

# honesty: policy gate present
if grep -q 'OODA_FS_WRITEDIR' runtime/chs_rt_fs.c \
  && grep -q 'oo_process_policy_getenv("OODA_FS_WRITEDIR")' runtime/chs_rt_fs.c \
  && grep -q 'path_under_writedir' runtime/chs_rt_fs.c \
  && grep -q 'O_NOFOLLOW' runtime/chs_rt_fs.c \
  && grep -q 'openat' runtime/chs_rt_fs.c; then
  pass "runtime writedir honesty (policy getenv + realpath + openat O_NOFOLLOW)"
else
  bad "runtime writedir honesty missing"
fi

PROBE="$TMPDIR/fs_wd_probe_$$.c"
BIN="$TMPDIR/fs_wd_probe_$$.bin"
cat >"$PROBE" <<'EOF'
#include "chs_rt.h"
#include <stdio.h>
int main(int argc, char **argv) {
  long long fs = oo_cap_grant_fs();
  const char *p = (argc > 1 && argv[1][0]) ? argv[1] : "/tmp/ooda_wd_missing";
  OoStr path = oo_str_lit(p);
  OoStr body = oo_str_lit("wd-ok");
  OoResV r = oo_write_file(fs, path, body);
  if (r.ok) {
    printf("write-ok\n");
    return 0;
  }
  printf("write-err:%s\n", r.err.data ? r.err.data : "");
  return 1;
}
EOF
gcc -O0 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$PROBE" -o "$BIN" -lm -lpthread -ldl \
  2>"$TMPDIR/fs_wd_probe.gcc" || {
  bad "gcc writedir probe"
  head -15 "$TMPDIR/fs_wd_probe.gcc" || true
}

WD="$TMPDIR/fs_wd_allow_$$"
OUT="$TMPDIR/fs_wd_out_$$"
mkdir -p "$WD" "$OUT"
NEW="$WD/created.txt"
EXIST="$WD/exist.txt"
OUTSIDE="$TMPDIR/fs_wd_outside_$$.txt"
echo pre >"$EXIST"
rm -f "$NEW" "$OUTSIDE"

if [[ -x "$BIN" ]]; then
  # fail-closed: unset
  out=$(env -u OODA_FS_WRITEDIR "$BIN" "$NEW" 2>&1) || true
  echo "$out" | grep -q 'write-err:.*OODA_FS_WRITEDIR' \
    && pass "unset OODA_FS_WRITEDIR deny" \
    || bad "unset out=$out"

  # fail-closed: empty
  out=$(OODA_FS_WRITEDIR= "$BIN" "$NEW" 2>&1) || true
  echo "$out" | grep -q 'write-err:.*OODA_FS_WRITEDIR' \
    && pass "empty OODA_FS_WRITEDIR deny" \
    || bad "empty out=$out"

  # root refused
  out=$(OODA_FS_WRITEDIR=/ "$BIN" "$NEW" 2>&1) || true
  echo "$out" | grep -q 'write-err:.*OODA_FS_WRITEDIR' \
    && pass "OODA_FS_WRITEDIR=/ refuse" \
    || bad "root out=$out"

  # allow create under dir
  out=$(OODA_FS_WRITEDIR="$WD" "$BIN" "$NEW" 2>&1) || true
  if echo "$out" | grep -q 'write-ok' && [[ -f "$NEW" ]] \
     && grep -q 'wd-ok' "$NEW"; then
    pass "allow create under OODA_FS_WRITEDIR"
  else
    bad "create under allow out=$out"
  fi

  # allow overwrite existing
  out=$(OODA_FS_WRITEDIR="$WD" "$BIN" "$EXIST" 2>&1) || true
  if echo "$out" | grep -q 'write-ok' && grep -q 'wd-ok' "$EXIST"; then
    pass "allow overwrite under OODA_FS_WRITEDIR"
  else
    bad "overwrite out=$out"
  fi

  # deny outside
  out=$(OODA_FS_WRITEDIR="$WD" "$BIN" "$OUTSIDE" 2>&1) || true
  if echo "$out" | grep -q 'write-err:.*OODA_FS_WRITEDIR' && [[ ! -f "$OUTSIDE" ]]; then
    pass "deny path outside writedir"
  else
    bad "outside out=$out"
  fi

  # deny .. escape (parent realpath leaves allowdir)
  esc="$WD/../fs_wd_escape_$$.txt"
  out=$(OODA_FS_WRITEDIR="$WD" "$BIN" "$esc" 2>&1) || true
  if echo "$out" | grep -q 'write-err:.*OODA_FS_WRITEDIR' && [[ ! -f "$esc" ]]; then
    pass "deny .. escape from writedir"
  else
    bad "escape out=$out"
  fi

  # leaf symlink to outside → deny (realpath or O_NOFOLLOW)
  echo secret >"$OUT/target.txt"
  ln -sfn "$OUT/target.txt" "$WD/leaf_link.txt"
  out=$(OODA_FS_WRITEDIR="$WD" "$BIN" "$WD/leaf_link.txt" 2>&1) || true
  if echo "$out" | grep -qE 'write-err' && grep -q 'secret' "$OUT/target.txt"; then
    pass "deny leaf symlink escape (content intact)"
  else
    bad "leaf symlink out=$out target=$(cat "$OUT/target.txt" 2>/dev/null || true)"
  fi

  # create via parent symlink to outside → deny (parent realpath outside)
  ln -sfn "$OUT" "$WD/outdir_link"
  out=$(OODA_FS_WRITEDIR="$WD" "$BIN" "$WD/outdir_link/via_parent.txt" 2>&1) || true
  if echo "$out" | grep -q 'write-err:.*OODA_FS_WRITEDIR' \
     && [[ ! -f "$OUT/via_parent.txt" ]]; then
    pass "deny create via parent symlink escape"
  else
    bad "parent symlink out=$out"
  fi
fi

rm -rf "$WD" "$OUT" "$BIN" "$PROBE" 2>/dev/null || true

if [[ $fail -ne 0 ]]; then
  echo "fs_writedir_smoke: FAILED" >&2
  exit 1
fi
echo "fs_writedir_smoke: PASSED"
exit 0
