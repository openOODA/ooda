#!/usr/bin/env bash
# job: pure-path ooda test — typecheck then run verify/contract fuzz harness
# in:  $1 = source .oo; OODAC_BIN or ./oodac/oodac; optional --fuzz [iters], --seed, --verbose
# out: exit 0 if check + tests pass; non-zero on fail (fail-closed)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export TMPDIR="${TMPDIR:-$HOME/.cache/ooda-tmp}"
mkdir -p "$TMPDIR"

SRC=""
FUZZ_MODE=0
FUZZ_ITERS=100
FUZZ_SEED=42
FUZZ_VERBOSE=0

args=("$@")
i=0
n=${#args[@]}

while [[ $i -lt $n ]]; do
  arg="${args[$i]}"
  if [[ "$arg" == "--fuzz" ]]; then
    FUZZ_MODE=1
    if [[ $((i + 1)) -lt $n ]]; then
      next_arg="${args[$((i + 1))]}"
      if [[ "$next_arg" != -* && "$next_arg" =~ ^[0-9]+$ ]]; then
        if [[ "$next_arg" -le 0 ]]; then
          echo "ERR	cli	invalid fuzz iterations: $next_arg" >&2
          exit 2
        fi
        FUZZ_ITERS="$next_arg"
        i=$((i + 1))
      elif [[ "$next_arg" == 0 || "$next_arg" =~ ^-[0-9]+$ ]]; then
        echo "ERR	cli	invalid fuzz iterations: $next_arg" >&2
        exit 2
      elif [[ "$next_arg" != -* && "$next_arg" != *.oo && ! -f "$next_arg" ]]; then
        echo "ERR	cli	invalid fuzz iterations: $next_arg" >&2
        exit 2
      fi
    fi
  elif [[ "$arg" == "--seed" ]]; then
    if [[ $((i + 1)) -lt $n ]]; then
      FUZZ_SEED="${args[$((i + 1))]}"
      i=$((i + 1))
    fi
  elif [[ "$arg" == "--verbose" ]]; then
    FUZZ_VERBOSE=1
  elif [[ "$arg" != -* ]]; then
    SRC="$arg"
  fi
  i=$((i + 1))
done

if [[ -z "$SRC" || ! -f "$SRC" ]]; then
  echo "ERR	test	unreadable file: ${SRC:-missing}" >&2
  exit 2
fi
if [[ "$SRC" != /* ]]; then
  SRC="$(pwd)/$SRC"
fi

OODAC="${OODAC_BIN:-}"
if [[ -z "$OODAC" || ! -x "$OODAC" ]]; then
  if [[ -x "$ROOT/oodac/oodac" ]]; then
    OODAC="$ROOT/oodac/oodac"
  elif [[ -x ./oodac/oodac ]]; then
    OODAC=./oodac/oodac
  else
    echo "ERR_NO_OODAC" >&2
    exit 1
  fi
fi

# --- 1) typecheck (fail-closed) ---
set +e
"$OODAC" check "$SRC" >"$TMPDIR/ooda_test_check.out" 2>"$TMPDIR/ooda_test_check.err"
ck=$?
set -e
if [[ $ck -ne 0 ]]; then
  cat "$TMPDIR/ooda_test_check.out" 2>/dev/null || true
  cat "$TMPDIR/ooda_test_check.err" >&2 2>/dev/null || true
  echo "ERR	test	check failed" >&2
  exit "$ck"
fi

# --- 2) lower verify blocks / contract fuzz loop → harness .oo ---
HARNESS="$TMPDIR/ooda_test_$$_harness.oo"
HARNESS_C="$TMPDIR/ooda_test_$$_harness.c"
OUTBIN="$TMPDIR/ooda_test_$$_bin"
cleanup_test() {
  if [[ -z "${OODA_TEST_KEEP:-}" ]]; then
    rm -f "$HARNESS" "$OUTBIN" "$HARNESS_C" "$HARNESS.c" 2>/dev/null || true
  fi
}
trap cleanup_test EXIT
export OODA_TEST_SRC="$SRC"
export OODA_TEST_HARNESS="$HARNESS"
export OODA_TEST_FUZZ="$FUZZ_MODE"
export OODA_TEST_FUZZ_ITERS="$FUZZ_ITERS"
export OODA_TEST_FUZZ_SEED="$FUZZ_SEED"
export OODA_TEST_FUZZ_VERBOSE="$FUZZ_VERBOSE"

if [[ $FUZZ_MODE -eq 1 ]]; then
  # Pure Int-domain path only — never python3 / ooda_fuzz_*.py / ooda_test_harness.py
  PURE="$ROOT/scripts/ooda_fuzz_pure.sh"
  if [[ ! -x "$PURE" ]]; then
    echo "ERR	test	missing pure fuzz generator: $PURE" >&2
    exit 1
  fi
  set +e
  "$PURE"
  pure_rc=$?
  set -e
  if [[ $pure_rc -ne 0 ]]; then
    exit "$pure_rc"
  fi
else
  PY="$ROOT/scripts/ooda_test_harness.py"
  if [[ ! -f "$PY" ]]; then
    echo "ERR	test	missing $PY" >&2
    exit 1
  fi
  set +e
  python3 "$PY"
  py=$?
  set -e
  if [[ $py -ne 0 ]]; then
    exit "$py"
  fi
fi

# No verify blocks and no fuzzing → check-only success
if [[ ! -s "$HARNESS" ]]; then
  exit 0
fi

# --- 3) build harness ---
if [[ $FUZZ_MODE -eq 1 ]]; then
  # Pure single-module link: emit-c + ARC decl inject + gcc (no python3 / pure_build)
  fuzz_emit() {
    local em="$1"
    # timeout absorbs SEGV without noisy shell job messages; fail-closed on non-zero.
    set +e
    EMIT_NO_CONCAT=1 timeout 60 "$em" emit-c "$HARNESS" \
      >"$HARNESS_C" 2>"$TMPDIR/ooda_test_build.err"
    local ec=$?
    set -e
    if [[ $ec -ne 0 || ! -s "$HARNESS_C" ]]; then
      return 1
    fi
    if ! grep -qE '^(void|int|long long|OoStr) ' "$HARNESS_C"; then
      return 1
    fi
    return 0
  }
  # Prefer seed for multi-fn harness stability when present; else tree OODAC.
  EMIT_OK=0
  if [[ -x "$ROOT/bootstrap/seed/oodac" ]] && fuzz_emit "$ROOT/bootstrap/seed/oodac"; then
    EMIT_OK=1
  elif fuzz_emit "$OODAC"; then
    EMIT_OK=1
  fi
  if [[ $EMIT_OK -ne 1 ]]; then
    cat "$TMPDIR/ooda_test_build.err" >&2 2>/dev/null || true
    echo "ERR	test	pure fuzz emit-c failed" >&2
    exit 1
  fi
  # Seed emit may use oo_*_release without decls — inject after OoSList typedef (bash only).
  TMPC="$TMPDIR/ooda_test_$$_harness_link.c"
  awk '
    { print }
    /} OoSList;/ && !done {
      print "void oo_slist_retain(OoSList); void oo_slist_release(OoSList);"
      print "void oo_ilist_retain(OoIList); void oo_ilist_release(OoIList);"
      print "void oo_str_retain(OoStr); void oo_str_release(OoStr);"
      done = 1
    }
  ' "$HARNESS_C" >"$TMPC"
  set +e
  gcc -O2 -I"$ROOT/runtime" "$ROOT/runtime/chs_rt.c" "$TMPC" -lm -o "$OUTBIN" \
    >"$TMPDIR/ooda_test_build.out" 2>"$TMPDIR/ooda_test_build.err"
  br=$?
  set -e
  rm -f "$TMPC" 2>/dev/null || true
  if [[ $br -ne 0 || ! -x "$OUTBIN" ]]; then
    cat "$TMPDIR/ooda_test_build.out" 2>/dev/null || true
    cat "$TMPDIR/ooda_test_build.err" >&2 2>/dev/null || true
    echo "ERR	test	pure fuzz harness gcc failed" >&2
    exit 1
  fi
else
  set +e
  OODAC_BIN="$OODAC" "$OODAC" build "$HARNESS" "$OUTBIN" \
    >"$TMPDIR/ooda_test_build.out" 2>"$TMPDIR/ooda_test_build.err"
  br=$?
  set -e
  if [[ $br -ne 0 || ! -x "$OUTBIN" ]]; then
    cat "$TMPDIR/ooda_test_build.out" 2>/dev/null || true
    cat "$TMPDIR/ooda_test_build.err" >&2 2>/dev/null || true
    echo "ERR	test	harness build failed" >&2
    exit 1
  fi
fi

# --- 4) run harness ---
set +e
"$OUTBIN" >"$TMPDIR/ooda_test_run.out" 2>"$TMPDIR/ooda_test_run.err"
rr=$?
set -e
cat "$TMPDIR/ooda_test_run.out" 2>/dev/null || true
if [[ $rr -ne 0 ]]; then
  cat "$TMPDIR/ooda_test_run.err" >&2 2>/dev/null || true
  exit "$rr"
fi

if [[ $FUZZ_MODE -eq 0 ]] && ! grep -q "OK verify" "$TMPDIR/ooda_test_run.out"; then
  echo "ERR	test	harness ran but missing OK verify" >&2
  exit 1
fi
exit 0
