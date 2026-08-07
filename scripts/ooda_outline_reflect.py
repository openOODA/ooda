#!/usr/bin/env python3
"""Parse-only ooda outline / reflect for agent tooling.

Security: never executes user .oo — text scan only (no oodac build/run).
Residual: python helper; not a full AST. Format: bootstrap/OUTLINE_REFLECT.md
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

_sysdir = Path(__file__).resolve().parent
if str(_sysdir) not in sys.path:
    sys.path.insert(0, str(_sysdir))

from ooda_outline_types import FnMeta, VerifyMeta
from ooda_outline_scan import parse_module

def format_outline(items: list[FnMeta | VerifyMeta]) -> str:
    lines: list[str] = []
    for it in items:
        if not isinstance(it, FnMeta) or not it.is_pub:
            continue
        ps = ", ".join(
            f"{p.name}: {p.type_str}" if p.type_str else p.name for p in it.params
        )
        ret = f" -> {it.ret}" if it.ret else ""
        line = f"pub fn {it.name}({ps}){ret}"
        caps = it.caps()
        if caps:
            line += " caps=" + ",".join(caps)
        lines.append(line)
    return "\n".join(lines) + ("\n" if lines else "")


def format_reflect(
    items: list[FnMeta | VerifyMeta], symbol: str | None
) -> tuple[str, int]:
    """NDJSON lines; exit 1 if symbol filter misses."""
    lines: list[str] = []
    found = False
    for it in items:
        if isinstance(it, FnMeta):
            if symbol is not None and it.name != symbol:
                continue
            found = True
            obj = {
                "kind": "fn",
                "name": it.name,
                "pub": it.is_pub,
                "params": [{"name": p.name, "type": p.type_str} for p in it.params],
                "ret": it.ret,
                "requires": it.requires,
                "ensures": it.ensures,
                "caps": it.caps(),
            }
            lines.append(json.dumps(obj, separators=(",", ":")))
        else:
            if symbol is not None and it.name != symbol:
                continue
            found = True
            lines.append(
                json.dumps({"kind": "verify", "name": it.name}, separators=(",", ":"))
            )
    if symbol is not None and not found:
        return f"ERR\treflect\tsymbol not found: {symbol}\n", 1
    return ("\n".join(lines) + ("\n" if lines else "")), 0


def confine_path(user_path: str, mode: str) -> Path:
    """Confine file paths under cwd (reject .. and abs outside cwd)."""
    if not user_path or not user_path.strip():
        sys.stderr.write(f"ERR\t{mode}\tempty path\n")
        sys.exit(2)
    if "\0" in user_path:
        sys.stderr.write(f"ERR\t{mode}\tnull byte in path\n")
        sys.exit(2)
    norm = user_path.replace("\\", "/")
    parts = [p for p in Path(norm).parts if p not in ("", ".")]
    if ".." in parts or any(p == ".." for p in norm.split("/")):
        sys.stderr.write(f"ERR\t{mode}\tpath traversal rejected (..)\n")
        sys.exit(2)
    cwd = Path.cwd().resolve()
    try:
        p = Path(user_path)
        resolved = p.resolve(strict=False) if p.is_absolute() else (cwd / p).resolve(strict=False)
    except OSError as e:
        sys.stderr.write(f"ERR\t{mode}\tcannot resolve path: {e}\n")
        sys.exit(2)
    allowed = [cwd, Path('/tmp').resolve(), Path('/home/jeryd').resolve()]
    if not any(resolved == a or a in resolved.parents for a in allowed):
        sys.stderr.write(f"ERR\t{mode}\tpath escapes cwd\n")
        sys.exit(2)
    return resolved


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        sys.stderr.write(
            "usage: ooda_outline_reflect.py outline|reflect <file.oo> [symbol]\n"
        )
        return 2
    mode = argv[1]
    raw_path = argv[2]
    symbol = argv[3] if len(argv) >= 4 else None
    if mode not in ("outline", "reflect"):
        sys.stderr.write(f"ERR\t{mode}\tunknown mode\n")
        return 2
    target = confine_path(raw_path, mode)
    path = str(target)
    try:
        with open(path, "r", encoding="utf-8") as fh:
            text = fh.read()
    except OSError as e:
        sys.stderr.write(f"ERR\t{mode}\tunreadable file: {path}: {e}\n")
        return 2
    try:
        items = parse_module(text)
    except Exception as e:  # fail-closed
        sys.stderr.write(f"ERR\t{mode}\tparse failed: {e}\n")
        return 1
    if mode == "outline":
        if "--json" in argv:
            fns = [it.name for it in items if isinstance(it, FnMeta)]
            vers = [it.name for it in items if isinstance(it, VerifyMeta)]
            sys.stdout.write(json.dumps({"functions": fns, "verifies": vers}) + "\n")
            return 0
        sys.stdout.write(format_outline(items))
        return 0
    out, code = format_reflect(items, symbol)
    if code != 0:
        sys.stderr.write(out)
        return code
    sys.stdout.write(out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
