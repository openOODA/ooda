#!/usr/bin/env bash
# job: compress gcc/oodac error logs into token-dense counts (no full dumps)
# in:  path to .err / log file, or stdin
# out: top error kinds + first 8 sample lines + file:line hits for top kind
set -euo pipefail

if [[ $# -ge 1 && -f "$1" ]]; then
  LOG="$1"
  DATA=$(cat "$LOG")
else
  DATA=$(cat)
fi

if [[ -z "${DATA// }" ]]; then
  echo "err_digest: empty"
  exit 0
fi

python3 - "$DATA" <<'PY'
import sys, re
from collections import Counter
data = sys.argv[1]
lines = data.splitlines()
kinds = Counter()
samples = []
file_hits = Counter()
for ln in lines:
    if "error:" in ln:
        m = re.search(r"error:\s*(.+)$", ln)
        kind = m.group(1).strip() if m else ln.strip()
        # collapse quoted names
        kind = re.sub(r"'[^']*'", "'…'", kind)
        kind = re.sub(r"\b[A-Za-z_][A-Za-z0-9_]*\b", lambda m: m.group(0) if m.group(0) in {
            "OoStr","OoSList","OoIList","OoResS","OoResV","long","int","void","struct","type"
        } else ("ID" if m.group(0)[0].islower() or m.group(0)[0]=='_' else "Name"), kind, count=3)
        kinds[kind[:120]] += 1
        if len(samples) < 8:
            samples.append(ln[:160])
        fm = re.search(r"([^/\s]+\.[ch]):(\d+):", ln)
        if fm:
            file_hits[f"{fm.group(1)}:{fm.group(2)}"] += 1
    elif "ERR\t" in ln or ln.startswith("ERR"):
        kinds[ln.strip()[:80]] += 1
        if len(samples) < 8:
            samples.append(ln[:160])

nerr = sum(kinds.values())
print(f"err_digest errors={nerr} unique_kinds={len(kinds)}")
print("top_kinds:")
for k, v in kinds.most_common(12):
    print(f"  {v:4d}  {k}")
if file_hits:
    print("hot_sites:")
    for k, v in file_hits.most_common(8):
        print(f"  {v:4d}  {k}")
if samples:
    print("samples:")
    for s in samples:
        print(f"  | {s}")
PY
