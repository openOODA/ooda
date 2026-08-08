#!/usr/bin/env python3
"""Multi-module typecheck without whole-program source concat (avoids OOM).

For each .oo in the import graph of <main>:
  - Build signature stubs for all OTHER modules' functions
  - Typecheck (stubs + this module body without import lines)

Exit 0 only if every module unit-checks clean.
"""
from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path

IMPORT_RE = re.compile(r'^\s*import\s+"([^"]+)"\s*;\s*$', re.M)
FN_RE = re.compile(
    r"(?:pub\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)"
    r"(?:\s*->\s*([A-Za-z_][A-Za-z0-9_\[\],\s]*))?\s*\{",
    re.M,
)


def collect(main: Path) -> list[Path]:
    seen: set[Path] = set()
    order: list[Path] = []
    stack: list[Path] = []

    def walk(p: Path) -> None:
        p = p.resolve()
        if p in stack:
            raise SystemExit(f"ERR_IMPORT_CYCLE {p}")
        if p in seen:
            return
        if not p.is_file():
            raise SystemExit(f"ERR_MISSING {p}")
        stack.append(p)
        text = p.read_text(encoding="utf-8", errors="replace")
        for m in IMPORT_RE.finditer(text):
            walk((p.parent / m.group(1)).resolve())
        stack.pop()
        seen.add(p)
        order.append(p)

    walk(main)
    return order


def strip_imports(text: str) -> str:
    return IMPORT_RE.sub("", text)


def fns_in(text: str) -> dict[str, tuple[str, str | None]]:
    out: dict[str, tuple[str, str | None]] = {}
    for m in FN_RE.finditer(text):
        name, params, ret = m.group(1), m.group(2), m.group(3)
        if name not in out:
            out[name] = (params, ret.strip() if ret else None)
    return out


def stub_body(ret: str | None) -> str:
    if not ret:
        return "{\n}\n"
    r = ret.strip()
    if r == "Void":
        return "{\n}\n"
    if r == "Bool":
        return "{\n    return false;\n}\n"
    if r == "Int" or r == "Float":
        return "{\n    return 0;\n}\n"
    if r.startswith("List"):
        return "{\n    return list_new();\n}\n"
    if r.startswith("Result"):
        return '{\n    return Err("stub");\n}\n'
    if r.startswith("Option"):
        return "{\n    return None;\n}\n"
    return '{\n    return "";\n}\n'


def sanitize_params(params: str) -> str:
    if not params.strip():
        return ""
    parts = [p.strip() for p in params.split(",") if p.strip()]
    out = []
    for idx, p in enumerate(parts):
        if ":" in p:
            _, ptype = p.split(":", 1)
            out.append(f"_p{idx}: {ptype.strip()}")
        else:
            out.append(f"_p{idx}: {p.strip()}")
    return ", ".join(out)


def build_stubs(all_fns: dict[str, tuple[str, str | None]], exclude: set[str]) -> str:
    lines = ["// signature stubs for multi-module unit check\n"]
    for name, (params, ret) in sorted(all_fns.items()):
        if name in exclude:
            continue
        san_p = sanitize_params(params)
        sig = f"pub fn {name}({san_p})"
        if ret:
            sig += f" -> {ret}"
        lines.append(sig + " " + stub_body(ret))
    return "\n".join(lines)



def main() -> int:
    if len(sys.argv) < 3:
        print("usage: oodac_module_check.py <main.oo> <oodac_bin>", file=sys.stderr)
        return 2
    main_oo = Path(sys.argv[1]).resolve()
    oodac = Path(sys.argv[2]).resolve()
    if not oodac.is_file():
        print(f"ERR_NO_OODAC {oodac}", file=sys.stderr)
        return 1
    mods = collect(main_oo)
    all_fns: dict[str, tuple[str, str | None]] = {}
    bodies: dict[Path, str] = {}
    local_names: dict[Path, set[str]] = {}
    for p in mods:
        text = p.read_text(encoding="utf-8", errors="replace")
        bodies[p] = strip_imports(text)
        local = fns_in(text)
        local_names[p] = set(local)
        for k, v in local.items():
            if k not in all_fns:
                all_fns[k] = v

    fails = 0
    with tempfile.TemporaryDirectory(prefix="oodac_mchk_") as td:
        td_path = Path(td)
        for mod in mods:
            stubs = build_stubs(all_fns, local_names[mod])
            unit = td_path / f"unit_{mod.stem}.oo"
            unit.write_text(stubs + "\n" + bodies[mod], encoding="utf-8")
            r = subprocess.run(
                [str(oodac), "check", str(unit)],
                cwd=str(td_path),
                capture_output=True,
                text=True,
            )
            if r.returncode != 0:
                fails += 1
                print(f"ERR_MODULE_CHECK {mod}", file=sys.stderr)
                sys.stderr.write(r.stdout or "")
                sys.stderr.write(r.stderr or "")
            else:
                print(f"OK_MODULE {mod.name}")
    if fails:
        print(f"ERR_MODULE_CHECK_TOTAL {fails}/{len(mods)}", file=sys.stderr)
        return 1
    print(f"OK_MODULE_CHECK nmods={len(mods)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
