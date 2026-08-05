#!/bin/sh
set -e
# scripts/oodac_concat.sh
# Concatenates a main .oo file and all its imports.

MAIN_FILE="$1"
DIR=$(dirname "$MAIN_FILE")

while IFS= read -r line; do
    if echo "$line" | grep -q '^import '; then
        IMPORT_NAME=$(echo "$line" | sed -n 's/^import "\(.*\)";/\1/p')
        if [ -n "$IMPORT_NAME" ]; then
            cat "$DIR/$IMPORT_NAME"
        else
            echo "$line"
        fi
    else
        echo "$line"
    fi
done < "$MAIN_FILE"
