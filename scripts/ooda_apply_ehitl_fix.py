#!/usr/bin/env python3
"""Bounded E_HITL auto-fix: structural remove exact `// HITL: pause` lines.

  ooda_apply_ehitl_fix.py <file.oo> [--dry-run]

Only lines whose stripped text is exactly `// HITL: pause` are removed.
Does not rewrite free-form comments. Non-applicable diags fail closed.
"""
from __future__ import annotations

import json, os, subprocess, sys
from pathlib import Path

PAUSE_EXACT = "// HITL: pause"


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


def strip_hitl_pause_lines(text: str) -> tuple[str, int]:
    """Remove lines that strip to exactly // HITL: pause. Returns (new, n_removed)."""
    out: list[str] = []
    removed = 0
    # preserve final newline style via join of original keepends pieces
    parts = text.splitlines(keepends=True)
    if not parts and text == "":
        return text, 0
    for ln in parts:
        core = ln.rstrip("\r\n")
        if core.strip() == PAUSE_EXACT:
            removed += 1
            continue
        out.append(ln)
    return "".join(out), removed


def main() -> None:
    if len(sys.argv) < 2:
        die("usage: ooda_apply_ehitl_fix.py <file.oo> [--dry-run]", 2)
    path = Path(sys.argv[1])
    dry = "--dry-run" in sys.argv
    if not path.is_file():
        die(f"missing file: {path}", 2)
    if ".." in path.parts:
        die("path traversal rejected", 2)

    oodac = resolve_oodac()
    diags, _ = json_errors(oodac, str(path))
    ehitl = [d for d in diags if d.get("code") == "E_HITL"]
    if not ehitl:
        die("no E_HITL diagnostic (non-applicable)")

    text = path.read_text(encoding="utf-8")
    fixed, n = strip_hitl_pause_lines(text)
    if n == 0 or fixed == text:
        die("no exact // HITL: pause line to remove (non-applicable)")

    if dry:
        sys.stdout.write(fixed)
        return

    path.write_text(fixed, encoding="utf-8")
    diags2, rc2 = json_errors(oodac, str(path))
    still = [d for d in diags2 if d.get("code") == "E_HITL"]
    if still:
        die(f"E_HITL still present after fix: {still[0].get('msg', '')[:120]}")
    print(f"OK\tfix\tapplied E_HITL remove {n} // HITL: pause line(s) from {path}")
    if rc2 != 0 and diags2:
        print("OK\tfix\tE_HITL cleared (other diags may remain)")
    raise SystemExit(0)


if __name__ == "__main__":
    main()
