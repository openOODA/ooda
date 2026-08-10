#!/usr/bin/env bash
# job: M119 shared LLVM IR → native binary via clang or llc+linker + chs_rt
# in:  [-O0|-O1|-O2|-O3] <in.ll> <out.bin>
# fail-closed: missing tools → ERR_NO_LLVM; never soft-pass
set -euo pipefail

usage() {
  echo "usage: $0 [-O0|-O1|-O2|-O3] <in.ll> <out.bin>" >&2
  exit 2
}

OPT="-O0"
while [[ $# -gt 0 ]]; do
  case "$1" in
    -O0|-O1|-O2|-O3) OPT="$1"; shift ;;
    -h|--help) usage ;;
    -*) echo "ERR_LLVM_LINK: unknown flag $1" >&2; usage ;;
    *) break ;;
  esac
done
[[ $# -eq 2 ]] || usage

IN_LL="$1"
OUT_BIN="$2"
if [[ ! -f "$IN_LL" || ! -s "$IN_LL" ]]; then
  echo "ERR_LLVM_LINK: missing or empty IR: $IN_LL" >&2
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RT_C="$ROOT/runtime/chs_rt.c"
RT_I="$ROOT/runtime"
if [[ ! -f "$RT_C" ]]; then
  echo "ERR_NO_RUNTIME: missing $RT_C" >&2
  exit 1
fi

CLANG=""; LLC=""; LINKER=""
command -v clang >/dev/null 2>&1 && CLANG="$(command -v clang)"
command -v llc >/dev/null 2>&1 && LLC="$(command -v llc)"
if [[ -n "$CLANG" ]]; then LINKER="$CLANG"
elif command -v gcc >/dev/null 2>&1; then LINKER="$(command -v gcc)"
elif command -v cc >/dev/null 2>&1; then LINKER="$(command -v cc)"
fi

if [[ -z "$CLANG" && -z "$LLC" ]]; then
  echo "ERR_NO_LLVM: need clang (prefer) or llc on PATH" >&2
  exit 1
fi
if [[ -z "$CLANG" && -n "$LLC" && -z "$LINKER" ]]; then
  echo "ERR_NO_LLVM: llc present but no clang/gcc/cc to link" >&2
  exit 1
fi

export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
WORKDIR="$TMPDIR/llvm_link_$$"
mkdir -p "$WORKDIR" "$(dirname "$OUT_BIN")"
trap 'rm -rf "$WORKDIR"' EXIT

CC_LOG="$WORKDIR/cc.out"; CC_ERR="$WORKDIR/cc.err"
set +e
if [[ -n "$CLANG" ]]; then
  timeout 60 "$CLANG" "$OPT" -Wno-override-module -Wno-unused-command-line-argument \
    -I"$RT_I" "$IN_LL" "$RT_C" -o "$OUT_BIN" -lm >"$CC_LOG" 2>"$CC_ERR"
  cc_ec=$?; CC_PATH="clang"
else
  OUT_OBJ="$WORKDIR/out.o"
  timeout 60 "$LLC" -filetype=obj -o "$OUT_OBJ" "$IN_LL" >"$WORKDIR/llc.out" 2>"$WORKDIR/llc.err"
  llc_ec=$?
  if [[ $llc_ec -ne 0 || ! -s "$OUT_OBJ" ]]; then
    echo "FAIL llc exit=$llc_ec" >&2; cat "$WORKDIR/llc.err" >&2 || true; exit 1
  fi
  timeout 60 "$LINKER" "$OPT" -I"$RT_I" "$OUT_OBJ" "$RT_C" -o "$OUT_BIN" -lm >"$CC_LOG" 2>"$CC_ERR"
  cc_ec=$?; CC_PATH="llc+$LINKER"
fi
set -e
if [[ $cc_ec -ne 0 || ! -x "$OUT_BIN" ]]; then
  echo "FAIL llvm compile/link via $CC_PATH $OPT exit=$cc_ec" >&2
  cat "$CC_ERR" >&2 || true; exit 1
fi
echo "OK llvm_link $OPT via $CC_PATH → $OUT_BIN" >&2
exit 0
