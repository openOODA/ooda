#!/bin/sh
# scripts/oodac_concat.sh — residual multi-file flatten for EMIT_CONCAT=1
# Prefer in-tree load (oodac/load_import.oo) for check|tokens|ast.
# Fail-closed: missing import → ERR_IMPORT_MISSING; cycle → ERR_IMPORT_CYCLE.
set -e

MAIN_FILE="$1"
if [ -z "$MAIN_FILE" ] || [ ! -f "$MAIN_FILE" ]; then
    echo "ERR_IMPORT_MISSING ${MAIN_FILE:-<empty>}" >&2
    exit 1
fi

# stack / done as newline-separated absolute-ish paths
STACK=""
DONE=""

abspath() {
    # $1 path relative to $2 dir
    _d="$2"
    _n="$1"
    case "$_n" in
        /*) echo "$_n" ;;
        *) echo "$_d/$_n" ;;
    esac
}

# Recursive expand: prints body to stdout
expand_file() {
    _path="$1"
    _stack="$2"

    case "$_stack" in
        *"$(printf '\n')$_path$(printf '\n')"*)
            echo "ERR_IMPORT_CYCLE $_path" >&2
            exit 1
            ;;
    esac
    case "$DONE" in
        *"$(printf '\n')$_path$(printf '\n')"*) return 0 ;;
    esac

    if [ ! -f "$_path" ]; then
        echo "ERR_IMPORT_MISSING $_path" >&2
        exit 1
    fi

    _stack2="${_stack}${_path}$(printf '\n')"
    _dir=$(dirname "$_path")
    # shellcheck disable=SC2162
    while IFS= read -r line || [ -n "$line" ]; do
        if echo "$line" | grep -q '^import "'; then
            IMPORT_NAME=$(echo "$line" | sed -n 's/^import "\(.*\)";.*/\1/p')
            if [ -z "$IMPORT_NAME" ]; then
                echo "$line"
            else
                _ip=$(abspath "$IMPORT_NAME" "$_dir")
                expand_file "$_ip" "$_stack2"
            fi
        else
            echo "$line"
        fi
    done < "$_path"
    DONE="${DONE}${_path}$(printf '\n')"
}

# Resolve MAIN to a concrete path
case "$MAIN_FILE" in
    /*) MAIN_ABS="$MAIN_FILE" ;;
    *) MAIN_ABS="$(pwd)/$MAIN_FILE" ;;
esac
# Prefer realpath when available
if command -v realpath >/dev/null 2>&1; then
    MAIN_ABS=$(realpath "$MAIN_FILE")
fi

expand_file "$MAIN_ABS" "$(printf '\n')"
