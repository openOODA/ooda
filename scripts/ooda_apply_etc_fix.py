#!/usr/bin/env python3
"""Bounded E_TC auto-fix: undefined variable → insert `let name = 0;` in fn body.

Structural only. Non-applicable diags fail closed.
  ooda_apply_etc_fix.py <file.oo> [--dry-run]
"""
from __future__ import annotations

import json, os, re, subprocess, sys
from pathlib import Path

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
        capture_output=True, text=True,
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

def extract_undefined_name(msg: str) -> str | None:
    m = re.search(r"undefined variable '([A-Za-z_][A-Za-z0-9_]*)'", msg)
    if m:
        return m.group(1)
    m = re.search(r"undefined variable \"([A-Za-z_][A-Za-z0-9_]*)\"", msg)
    if m:
        return m.group(1)
    return None

def find_fn_body_open(text: str, line_hint: int) -> int | None:
    """Byte index of '{' opening the fn body containing line_hint (1-based)."""
    lines = text.splitlines(keepends=True)
    idx = max(0, min(len(lines), line_hint) - 1)
    start_line = idx
    for i in range(idx, -1, -1):
        if re.search(r"\bfn\b", lines[i]) and not lines[i].lstrip().startswith("//"):
            start_line = i
            break
    else:
        return None
    abs_start = sum(len(lines[i]) for i in range(start_line))
    chunk = text[abs_start:]
    m = re.match(r"(?:pub\s+)?fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\(", chunk)
    if not m:
        return None
    j = abs_start + m.end() - 1
    # skip params
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
                k += 1
                break
        k += 1
    while k < n and text[k] != "{":
        k += 1
    if k >= n or text[k] != "{":
        return None
    return k

def apply_undefined_let(text: str, name: str, line_hint: int) -> str:
    if re.search(rf"\blet\s+(?:mut\s+)?{re.escape(name)}\b", text):
        die(f"let {name} already present (non-applicable)")
    open_b = find_fn_body_open(text, line_hint)
    if open_b is None:
        die("could not find enclosing fn body")
    insert = f"\n    let {name} = 0;"
    return text[: open_b + 1] + insert + text[open_b + 1 :]

def main() -> None:
    if len(sys.argv) < 2:
        die("usage: ooda_apply_etc_fix.py <file.oo> [--dry-run]", 2)
    path = Path(sys.argv[1])
    dry = "--dry-run" in sys.argv
    if not path.is_file():
        die(f"missing file: {path}", 2)
    if ".." in path.parts:
        die("path traversal rejected", 2)

    oodac = resolve_oodac()
    diags, _ = json_errors(oodac, str(path))
    etc = [d for d in diags if d.get("code") == "E_TC"]
    if not etc:
        die("no E_TC diagnostic (non-applicable)")

    name = None
    line_hint = 1
    for d in etc:
        name = extract_undefined_name(d.get("msg") or "")
        if name:
            line_hint = int(d.get("line") or 1)
            break
    if not name:
        die("no undefined-variable E_TC (non-applicable)")

    text = path.read_text(encoding="utf-8")
    fixed = apply_undefined_let(text, name, line_hint)
    if fixed == text:
        die("fix produced no change")

    if dry:
        sys.stdout.write(fixed)
        return

    path.write_text(fixed, encoding="utf-8")
    diags2, _ = json_errors(oodac, str(path))
    still = []
    for d in diags2:
        if d.get("code") == "E_TC":
            n2 = extract_undefined_name(d.get("msg") or "")
            if n2 == name:
                still.append(d)
    if still:
        die(f"E_TC undefined '{name}' still present after fix")
    print(f"OK\tfix\tapplied E_TC undefined-var let {name}=0 to {path}")
    raise SystemExit(0)

if __name__ == "__main__":
    main()
