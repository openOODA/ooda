#!/bin/sh
set -u
TARGET="${1:-}"
OUTP="${2:-/tmp/oodac_json_errors_out.txt}"
ECP="${3:-/tmp/oodac_json_errors_ec.txt}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
OODAC="${OODAC_BIN:-$ROOT_DIR/oodac/oodac}"

if [ ! -x "$OODAC" ]; then
    OODAC="oodac"
fi

"$OODAC" check "$TARGET" > "$OUTP" 2>&1
EC=$?
echo "$EC" > "$ECP"
exit "$EC"
