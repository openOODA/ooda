#!/usr/bin/env python3
"""Surgical .oo replace_fn. Never shell-eval body. Structured op only; reject ..; atomic write.
  ooda_patch.py <file.oo> --replace-fn <n> --with <body> [--check]
  ooda_patch.py <file.oo> [--check]  # JSON stdin replace_fn
"""
from __future__ import annotations

import json, os, re, subprocess, sys, tempfile
from pathlib import Path

def die(msg: str, code: int = 2) -> None:
    print(f"ERR\tpatch\t{msg}", file=sys.stderr)
    raise SystemExit(code)

def confine(user_path: str, label: str) -> Path:
    """Confine file + body paths under cwd (reject .. and abs outside cwd)."""
    if not user_path or not user_path.strip():
        die(f"{label}: empty path")
    if "\0" in user_path:
        die(f"{label}: null byte in path")
    norm = user_path.replace("\\", "/")
    parts = [p for p in Path(norm).parts if p not in ("", ".")]
    if ".." in parts or any(p == ".." for p in norm.split("/")):
        die(f"{label}: path traversal rejected (..)")
    cwd = Path.cwd().resolve()
    try:
        p = Path(user_path)
        resolved = p.resolve(strict=False) if p.is_absolute() else (cwd / p).resolve(strict=False)
    except OSError as e:
        die(f"{label}: cannot resolve path: {e}")
    try:
        resolved.relative_to(cwd)
    except ValueError:
        die(f"{label}: path escapes cwd")
    return resolved

def _skip_string(t: str, j: int) -> int:
    j += 1
    n = len(t)
    while j < n:
        if t[j] == "\\":
            j += 2
            continue
        if t[j] == '"':
            return j + 1
        j += 1
    die("unclosed string")
    return j

def _skip_comment(t: str, j: int) -> int:
    if t.startswith("//", j):
        while j < len(t) and t[j] != "\n":
            j += 1
        return j
    end = t.find("*/", j + 2)
    if end < 0:
        die("unclosed block comment")
    return end + 2

def _skip_balanced(t: str, j: int, oc: str, cc: str) -> int:
    n = len(t)
    if j >= n or t[j] != oc:
        die(f"expected {oc}")
    depth = 0
    while j < n:
        c = t[j]
        if c == '"':
            j = _skip_string(t, j)
            continue
        if c == "/" and j + 1 < n and t[j + 1] in "/*":
            j = _skip_comment(t, j)
            continue
        if c == oc:
            depth += 1
        elif c == cc:
            depth -= 1
            j += 1
            if depth == 0:
                return j
            continue
        j += 1
    die("unbalanced braces/parens")
    return j

def find_fn_bodies(text: str, name: str) -> list[tuple[int, int]]:
    n, hits, i = len(text), [], 0
    while i < n:
        if text.startswith("//", i) or text.startswith("/*", i):
            i = _skip_comment(text, i)
            continue
        if text[i] == '"':
            i = _skip_string(text, i)
            continue
        m = re.match(r"(?:pub\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\b", text[i:])
        if not m:
            i += 1
            continue
        fname, j = m.group(1), i + m.end()
        while j < n and text[j] in " \t\r\n":
            j += 1
        if j >= n or text[j] != "(":
            i += 1
            continue
        j = _skip_balanced(text, j, "(", ")")
        while j < n and text[j] != "{":
            if text.startswith("//", j) or text.startswith("/*", j):
                j = _skip_comment(text, j)
                continue
            if text[j] == '"':
                j = _skip_string(text, j)
                continue
            j += 1
        if j >= n or text[j] != "{":
            die(f"fn {fname}: missing body")
        open_b, after = j, _skip_balanced(text, j, "{", "}")
        if fname == name:
            hits.append((open_b, after))
        i = after
    return hits

def replace_fn(text: str, name: str, body: str) -> str:
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name or ""):
        die(f"invalid function name: {name!r}")
    hits = find_fn_bodies(text, name)
    if not hits:
        die(f"function not found: {name}")
    if len(hits) > 1:
        die(f"ambiguous function: {name} ({len(hits)} definitions)")
    open_b, after = hits[0]
    b = body.strip("\n")
    if b.startswith("{") and b.endswith("}"):
        b = b[1:-1]
    lines = b.splitlines()
    if lines and not all(ln.startswith("    ") or not ln.strip() for ln in lines):
        lines = [("    " + ln if ln.strip() else ln) for ln in lines]
    inner = "\n".join(lines)
    if inner and not inner.startswith("\n"):
        inner = "\n" + inner
    if not inner.endswith("\n"):
        inner += "\n"
    return text[: open_b + 1] + inner + text[after - 1 :]

def atomic_write(path: Path, content: str) -> None:
    d = path.parent if str(path.parent) not in ("", ".") else Path(".")
    fd, tmp = tempfile.mkstemp(prefix=path.name + ".", suffix=".tmp", dir=str(d))
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(content)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, path)
    except Exception:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise

def parse_cli(argv: list[str]) -> tuple[Path, str, str, bool]:
    if not argv:
        die("usage: ooda_patch.py <file.oo> --replace-fn <n> --with <body> [--check]")
    target = confine(argv[0], "target")
    name, body_file, check, i = "", "", False, 1
    while i < len(argv):
        a = argv[i]
        if a == "--replace-fn":
            i += 1
            if i >= len(argv):
                die("--replace-fn needs name")
            name = argv[i]
        elif a == "--with":
            i += 1
            if i >= len(argv):
                die("--with needs body file path")
            body_file = argv[i]
        elif a == "--check":
            check = True
        elif a not in ("--json", "-"):
            die(f"unknown arg: {a}")
        i += 1
    if name and body_file:
        bp = confine(body_file, "body")
        if not bp.is_file():
            die(f"body file not found: {body_file}")
        return target, name, bp.read_text(encoding="utf-8"), check
    if name or body_file:
        die("need both --replace-fn and --with, or JSON stdin only")
    if sys.stdin.isatty():
        die("JSON stdin required when flags omitted")
    try:
        doc = json.loads(sys.stdin.read())
    except json.JSONDecodeError as e:
        die(f"invalid JSON: {e}")
    if not isinstance(doc, dict):
        die("JSON must be an object")
    if doc.get("op") != "replace_fn":
        die(f"unsupported op {doc.get('op')!r} (only replace_fn)")
    for k in doc:
        if k not in ("op", "name", "body"):
            die(f"unknown JSON field: {k}")
    body = doc.get("body")
    if not isinstance(body, str):
        die("JSON replace_fn needs string body")
    return target, doc.get("name") or "", body, check

def run_check(path: Path) -> None:
    root = Path(__file__).resolve().parent.parent
    oodac = os.environ.get("OODAC_BIN") or ""
    cands = [oodac, str(root / "oodac" / "oodac"), "oodac/oodac", "./oodac/oodac"]
    em = next((c for c in cands if c and os.path.isfile(c) and os.access(c, os.X_OK)), "")
    if not em:
        die("OODAC_BIN / oodac missing for --check", 1)
    r = subprocess.run([em, "check", str(path)], capture_output=True, text=True)
    sys.stdout.write(r.stdout)
    sys.stderr.write(r.stderr)
    if r.returncode != 0:
        die("check failed after patch", r.returncode or 1)

def main() -> int:
    target, name, body, check = parse_cli(sys.argv[1:])
    if not target.is_file():
        die(f"file not found: {target}")
    if target.suffix != ".oo":
        die("target must be a .oo file")
    text = target.read_text(encoding="utf-8")
    new = replace_fn(text, name, body)
    if new == text:
        die("patch produced no change (identical body?)")
    atomic_write(target, new)
    print(f"OK\tpatch\treplace_fn {name} → {target}")
    if check:
        run_check(target)
    return 0

if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SystemExit:
        raise
    except Exception as e:
        print(f"ERR\tpatch\t{e}", file=sys.stderr)
        raise SystemExit(1)
