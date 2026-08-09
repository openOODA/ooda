"""Seed softener: strip formal-param oo_*_release (caller-owned)."""
from __future__ import annotations
import re

def _scan_c_braces(t: str, start: int) -> int:
    brace_count = 1
    curr = start
    n = len(t)
    in_str = in_chr = escape = False
    while curr < n and brace_count > 0:
        ch = t[curr]
        if escape:
            escape = False
            curr += 1
            continue
        if in_str:
            if ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
            curr += 1
            continue
        if in_chr:
            if ch == "\\":
                escape = True
            elif ch == "'":
                in_chr = False
            curr += 1
            continue
        if ch == '"':
            in_str = True
            curr += 1
            continue
        if ch == "'":
            in_chr = True
            curr += 1
            continue
        if ch == "{":
            brace_count += 1
        elif ch == "}":
            brace_count -= 1
        curr += 1
    return curr

def strip_formal_param_releases(t: str) -> str:
    """Remove oo_*_release(formal) that seed emit inserts (params are caller-owned).

    Keeps local reassign releases (__tmp) and body locals not in formals.
    Required before runtime free is safe for seed-emitted pure multi of oodac.
    """
    header_pattern = re.compile(
        r"^[ \t]*(?:static\s+)?(?:inline\s+)?(?:void|int|long\s+long|OoStr|OoSList|OoIList|OoResS|OoResV)\s+([A-Za-z0-9_]+)\s*\(([^)]*)\)\s*\{",
        re.MULTILINE,
    )
    out: list[str] = []
    pos = 0
    for m in header_pattern.finditer(t):
        start_fn = m.start()
        formals_raw = m.group(2).strip()
        formals: set[str] = set()
        if formals_raw and formals_raw != "void":
            for part in formals_raw.split(","):
                part = part.strip()
                if not part:
                    continue
                # last token is the name (OoStr set / long long x)
                name = part.split()[-1].strip()
                name = name.lstrip("*")
                if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name):
                    formals.add(name)
        curr = _scan_c_braces(t, m.end())
        if start_fn > pos:
            out.append(t[pos:start_fn])
        fn_text = t[start_fn:curr]
        if formals:
            def strip_rel(rm: re.Match[str]) -> str:
                var = rm.group(2)
                if var in formals:
                    return "/* no release formal " + var + " */"
                return rm.group(0)

            fn_text = re.sub(
                r"\b(oo_str_release|oo_slist_release|oo_ilist_release)\s*\(\s*([A-Za-z0-9_]+)\s*\)\s*;",
                strip_rel,
                fn_text,
            )
        out.append(fn_text)
        pos = curr
    if pos < len(t):
        out.append(t[pos:])
    return "".join(out)


