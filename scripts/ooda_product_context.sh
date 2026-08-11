#!/usr/bin/env bash
# job: ooda product CLI context backend (token-minimal symbol orient)
# in:  <file.oo> [symbol]   or file#symbol
# out: stdout JSON {symbol,file,context,outline_hint,est_tokens}
# Prefer this over cat'ing whole modules into agent context.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OODAC_BIN="${OODAC_BIN:-$ROOT/oodac/oodac}"

raw="${1:-}"
sym="${2:-}"
if [[ "$raw" == *"#"* ]]; then
  file="${raw%%#*}"
  sym="${raw#*#}"
else
  file="$raw"
fi
if [[ -z "$file" ]]; then
  echo '{"error":"usage: ooda_product_context.sh <file.oo> [symbol]"}'
  exit 2
fi
if [[ ! -f "$file" && -f "$ROOT/$file" ]]; then file="$ROOT/$file"; fi
if [[ ! -f "$file" ]]; then
  echo "{\"error\":\"missing file\",\"file\":\"$file\"}"
  exit 1
fi

python3 - "$file" "$sym" "$OODAC_BIN" <<'PY'
import json, sys, re, subprocess, os
path, sym, oodac = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(path).read()
lines = text.splitlines()
bytes_ = len(text.encode())
est = (bytes_ + 3) // 4
ctx = "no symbol requested"
slice_lines = []
if sym:
    pat = re.compile(rf'^(pub\s+)?(fn|type|struct|let)\s+{re.escape(sym)}\b')
    start = None
    for i, ln in enumerate(lines):
        if pat.search(ln.strip()) or re.search(rf'\bfn\s+{re.escape(sym)}\b', ln):
            start = i
            break
    if start is None:
        ctx = f"symbol '{sym}' not found in {path}"
    else:
        depth = 0
        seen = False
        for j in range(start, min(len(lines), start + 60)):
            ln = lines[j]
            slice_lines.append(ln)
            depth += ln.count("{") - ln.count("}")
            if "{" in ln:
                seen = True
            if seen and depth <= 0:
                break
            if len(slice_lines) >= 40:
                slice_lines.append("/* truncated */")
                break
        ctx = "\n".join(slice_lines)

outline = ""
if os.path.isfile(oodac) and os.access(oodac, os.X_OK):
    try:
        r = subprocess.run([oodac, "outline", path], capture_output=True, text=True, timeout=15)
        outline = (r.stdout or "")[:2000]
    except Exception as e:
        outline = f"(outline failed: {e})"

out = {
    "file": path,
    "symbol": sym,
    "file_lines": len(lines),
    "file_bytes": bytes_,
    "est_tokens_full": est,
    "est_tokens_slice": (len(ctx) + 3) // 4,
    "context": ctx,
    "outline": outline,
    "hint": "prefer outline/reflect; avoid full file when lines>80",
}
print(json.dumps(out, ensure_ascii=False))
PY
