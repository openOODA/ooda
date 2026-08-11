#!/usr/bin/env python3
"""Product ooda fix dispatcher: E_CAP structural then E_TC undefined-var.

  ooda_apply_fix.py <file.oo> [--dry-run]
"""
from __future__ import annotations

import json, os, subprocess, sys
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
    die("no oodac", 2)

def main() -> None:
    if len(sys.argv) < 2:
        die("usage: ooda_apply_fix.py <file.oo> [--dry-run]", 2)
    path = sys.argv[1]
    extra = sys.argv[2:]
    root = Path(__file__).resolve().parent
    oodac = resolve_oodac()
    p = subprocess.run(
        [oodac, "check", path, "--json-errors"],
        capture_output=True, text=True,
    )
    raw = (p.stdout or "") + "\n" + (p.stderr or "")
    lines = [ln for ln in raw.splitlines() if ln.strip().startswith("[")]
    codes = []
    if lines:
        try:
            v = json.loads(lines[-1])
            codes = [d.get("code") for d in v if isinstance(d, dict)]
        except json.JSONDecodeError:
            die("invalid json-errors")
    if "E_CAP" in codes:
        r = subprocess.run([sys.executable, str(root / "ooda_apply_ecap_fix.py"), path, *extra])
        raise SystemExit(r.returncode)
    if "E_TC" in codes:
        r = subprocess.run([sys.executable, str(root / "ooda_apply_etc_fix.py"), path, *extra])
        raise SystemExit(r.returncode)
    if p.returncode == 0:
        die("check passed (nothing to fix)")
    die("no supported diagnostic class (want E_CAP or E_TC undefined-var)")

if __name__ == "__main__":
    main()
