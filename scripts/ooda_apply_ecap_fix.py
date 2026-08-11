#!/usr/bin/env python3
"""Bounded E_CAP auto-fix: add missing &Cap param + first-arg token to sealed call.

Structural only — never shell-evals diagnostic text.
  ooda_apply_ecap_fix.py <file.oo> [--dry-run]

Applies only when:
  - oodac/ooda check --json-errors reports E_CAP
  - body has a bare sealed free call without a matching cap first arg
  - function lacks a matching &CapType param
Fails closed otherwise (exit 1).
"""
from __future__ import annotations

import json, os, re, subprocess, sys
from pathlib import Path

# Sealed free-call → required Cap type (mirrors check_cap_util)
SEALED = {
    "fetch": "NetCap", "downloadData": "NetCap", "http_get": "NetCap",
    "net_get": "NetCap", "net_connect": "NetCap", "query_remote_api": "NetCap",
    "read_file": "FsCap", "write_file": "FsCap", "fs_read": "FsCap",
    "fs_write": "FsCap", "path_exists": "FsCap", "file_size": "FsCap",
    "sys_exec": "SysCap", "exec": "SysCap", "spawn_process": "SysCap",
    "env_get": "EnvCap", "env_set": "EnvCap", "getenv": "EnvCap",
    "now_ms": "TimeCap", "sleep_ms": "TimeCap",
    "random": "RandCap", "seed": "RandCap",
    "alloc_bytes": "AllocCap", "free_bytes": "AllocCap",
    "malloc": "AllocCap", "free": "AllocCap", "realloc": "AllocCap",
}

PARAM_NAME = {
    "NetCap": "net", "FsCap": "fs", "SysCap": "sys", "EnvCap": "env",
    "TimeCap": "time", "RandCap": "rand", "AllocCap": "alloc",
}

def die(msg: str, code: int = 1) -> None:
    print(f"ERR\tfix\t{msg}", file=sys.stderr)
    raise SystemExit(code)

def resolve_oodac() -> str:
    for c in (
        os.environ.get("OODAC_BIN", ""),
        str(Path(__file__).resolve().parent.parent / "oodac" / "oodac"),
        "./oodac/oodac",
    ):
        if c and Path(c).is_file() and os.access(c, os.X_OK):
            return c
    die("no oodac binary (set OODAC_BIN)", 2)

def json_errors(oodac: str, path: str) -> tuple[list, int]:
    p = subprocess.run(
        [oodac, "check", path, "--json-errors"],
        capture_output=True, text=True
    )
    raw = (p.stdout or "") + "\n" + (p.stderr or "")
    lines = [ln for ln in raw.splitlines() if ln.strip().startswith("[")]
    if not lines:
        if p.returncode == 0:
            return [], 0
        die("no JSON diagnostics from check")
    try:
        return json.loads(lines[-1]), p.returncode
    except json.JSONDecodeError as e:
        die(f"invalid json-errors: {e}")

def has_cap_param(sig: str, cap: str) -> bool:
    # match &Cap or Cap in param list
    return bool(re.search(rf"&\s*{re.escape(cap)}\b|{re.escape(cap)}\b", sig))

def find_fn_header(text: str, line_hint: int) -> tuple[int, int, str] | None:
    """Return (header_start, open_paren_end_after_params, full_header_match) for fn near line."""
    lines = text.splitlines(keepends=True)
    # line_hint is 1-based from diag; search outward for fn
    idx = max(0, min(len(lines), line_hint) - 1)
    # search up for fn line
    start_line = idx
    for i in range(idx, -1, -1):
        if re.search(r"\bfn\b", lines[i]) and not lines[i].lstrip().startswith("//"):
            start_line = i
            break
    else:
        return None
    # accumulate from start_line until we have balanced params )
    chunk = "".join(lines[start_line:])
    m = re.match(
        r"((?:pub\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*)\s*\(",
        chunk,
    )
    if not m:
        return None
    abs_start = sum(len(lines[i]) for i in range(start_line))
    j = abs_start + m.end() - 1  # at '('
    # skip balanced parens for params
    n, depth, k = len(text), 0, j
    while k < n:
        c = text[k]
        if c == '"':
            k += 1
            while k < n and text[k] != '"':
                if text[k] == "\\":
                    k += 2
                    continue
                k += 1
            k += 1
            continue
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return abs_start, k + 1, text[abs_start : k + 1]
        k += 1
    return None

def find_bare_call(text: str, op: str) -> tuple[int, int] | None:
    """Find op( that is not already op(ident,  — return (op_start, after_lparen)."""
    for m in re.finditer(rf"\b{re.escape(op)}\s*\(", text):
        after = m.end()
        # skip whitespace
        k = after
        while k < len(text) and text[k] in " \t\n\r":
            k += 1
        # if already starts with cap-like first arg: word then comma
        m2 = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*,", text[k:])
        if m2:
            # already has first ident arg — skip (may already be fixed)
            continue
        return m.start(), after
    return None

def apply_fix(text: str, cap: str) -> str:
    op = None
    for name, c in SEALED.items():
        if c == cap and find_bare_call(text, name):
            op = name
            break
    if not op:
        # any sealed bare call matching cap
        for name, c in SEALED.items():
            if c != cap:
                continue
            hit = find_bare_call(text, name)
            if hit:
                op = name
                break
    if not op:
        die(f"no bare sealed call for {cap} to fix")

    hit = find_bare_call(text, op)
    if not hit:
        die(f"no bare {op}( call")
    op_start, after_lp = hit
    pname = PARAM_NAME[cap]

    # Insert first arg after '('
    text2 = text[:after_lp] + pname + ", " + text[after_lp:]

    # Find enclosing fn header — use line of the call
    line_no = text2.count("\n", 0, op_start) + 1
    hdr = find_fn_header(text2, line_no)
    if not hdr:
        die("could not find enclosing fn header")
    h0, h1, header = hdr
    if has_cap_param(header, cap):
        # param already there — only call fix needed (already done)
        return text2

    # Insert param before closing )
    # header ends with )
    inner = header[:-1]  # drop )
    # find open (
    lp = inner.rfind("(")
    params = inner[lp + 1 :].strip()
    new_param = f"{pname}: &{cap}"
    if params == "":
        new_params = new_param
    else:
        new_params = params + ", " + new_param
    new_header = inner[: lp + 1] + new_params + ")"
    text3 = text2[:h0] + new_header + text2[h1:]
    return text3

def main() -> None:
    if len(sys.argv) < 2:
        die("usage: ooda_apply_ecap_fix.py <file.oo> [--dry-run]", 2)
    path = Path(sys.argv[1])
    dry = "--dry-run" in sys.argv
    if not path.is_file():
        die(f"missing file: {path}", 2)
    # path safety: reject ..
    if ".." in path.parts:
        die("path traversal rejected", 2)

    oodac = resolve_oodac()
    diags, _rc0 = json_errors(oodac, str(path))
    ecap = [d for d in diags if d.get("code") == "E_CAP"]
    if not ecap:
        die("no E_CAP diagnostic (non-applicable)")

    # Infer cap: sealed op name in msg first, then explicit &Cap in msg.
    # Do NOT scan fix_hint first — it lists every &XCap and would always pick NetCap.
    msg = ecap[0].get("msg", "") or ""
    cap = None
    # Prefer longer op names first (e.g. path_exists before path)
    for op, c in sorted(SEALED.items(), key=lambda kv: -len(kv[0])):
        if re.search(rf"\b{re.escape(op)}\b", msg):
            cap = c
            break
    if not cap:
        for c in ("NetCap", "FsCap", "SysCap", "EnvCap", "TimeCap", "RandCap", "AllocCap"):
            if f"&{c}" in msg or re.search(rf"\b{re.escape(c)}\b", msg):
                cap = c
                break
    if not cap:
        die("could not infer Cap type from E_CAP msg (non-applicable)")

    text = path.read_text(encoding="utf-8")
    fixed = apply_fix(text, cap)
    if fixed == text:
        die("fix produced no change")

    if dry:
        sys.stdout.write(fixed)
        return

    path.write_text(fixed, encoding="utf-8")
    diags2, rc2 = json_errors(oodac, str(path))
    still = [d for d in diags2 if d.get("code") == "E_CAP"]
    if still:
        die(f"E_CAP still present after fix: {still[0].get('msg','')[:120]}")
    print(f"OK\tfix\tapplied E_CAP {cap} structural fix to {path}")
    if rc2 != 0 and diags2:
        print("OK\tfix\tE_CAP cleared (other diags may remain)")
    raise SystemExit(0)

if __name__ == "__main__":
    main()
