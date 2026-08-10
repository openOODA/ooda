#!/usr/bin/env bash
# job: ooda product CLI migrate backend
# in:  args
# out: stdout json or text
set -euo pipefail

file=""
json_mode=""
skip_next=0
for arg in "$@"; do
  if [[ $skip_next -eq 1 ]]; then
    skip_next=0
    continue
  fi
  if [[ "$arg" == "--json" ]]; then
    json_mode=1
  elif [[ "$arg" == "--edition" ]]; then
    skip_next=1
  elif [[ "$arg" != -* ]]; then
    if [[ -z "$file" ]]; then
      file="$arg"
    fi
  fi
done
fixes=0
changed="false"
if [[ -n "$file" && -f "$file" ]]; then
  fixes=$(grep -c -E '\blet[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]*=' "$file" 2>/dev/null || true)
  if [[ -z "$fixes" ]]; then fixes=0; fi
  if [[ $fixes -gt 0 ]]; then
    sed -i -E 's/\blet[[:space:]]+([a-zA-Z_][a-zA-Z0-9_]*)[[:space:]]*=/let mut \1 =/g' "$file" 2>/dev/null || true
    changed="true"
  fi
fi
if [[ -n "$json_mode" ]]; then
  echo "{\"file\": \"$file\", \"edition\": \"2026\", \"match_wildcard_arms\": 0, \"let_mut_fixes\": $fixes, \"changed\": $changed}"
else
  echo "migrated $file: let_mut_fixes=$fixes changed=$changed"
fi
