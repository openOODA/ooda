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
OUTBIN="$TMPDIR/ooda_test_$$_bin"
cleanup_test() {
  if [[ -z "${OODA_TEST_KEEP:-}" ]]; then
    rm -f "$HARNESS" "$OUTBIN" "$HARNESS.c" 2>/dev/null || true
  fi
}
trap cleanup_test EXIT
export OODA_TEST_SRC="$SRC"
export OODA_TEST_HARNESS="$HARNESS"
export OODA_TEST_FUZZ="$FUZZ_MODE"
export OODA_TEST_FUZZ_ITERS="$FUZZ_ITERS"
export OODA_TEST_FUZZ_SEED="$FUZZ_SEED"
export OODA_TEST_FUZZ_VERBOSE="$FUZZ_VERBOSE"

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

# No verify blocks and no fuzzing → check-only success
if [[ ! -s "$HARNESS" ]]; then
  exit 0
fi

# --- 3) build harness ---
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
