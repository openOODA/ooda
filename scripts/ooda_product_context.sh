#!/usr/bin/env bash
# job: ooda product CLI context backend
# in:  args
# out: stdout json
set -euo pipefail

file=""
sym=""
for arg in "$@"; do
  if [[ "$arg" != -* ]]; then
    if [[ -z "$file" ]]; then
      file="$arg"
    elif [[ -z "$sym" ]]; then
      sym="$arg"
    fi
  fi
done
ctx=""
if [[ -n "$file" && -f "$file" && -n "$sym" ]]; then
  match_line=$(grep -n -E "\b(fn|let|struct|enum|type)\s+${sym}\b" "$file" 2>/dev/null | head -n 1 || true)
  if [[ -n "$match_line" ]]; then
    ctx=$(echo "$match_line" | tr '\n' ' ' | sed 's/"/\\"/g')
  else
    ctx="symbol '$sym' definition not found in $file"
  fi
else
  ctx="no context definition"
fi
echo "{\"symbol\": \"$sym\", \"file\": \"$file\", \"context\": \"$ctx\"}"
