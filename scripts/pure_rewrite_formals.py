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


def _parse_formals(formals_raw: str) -> set[str]:
    formals: set[str] = set()
    if not formals_raw or formals_raw == "void":
        return formals
    for part in formals_raw.split(","):
        part = part.strip()
        if not part:
            continue
        name = part.split()[-1].strip().lstrip("*")
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name):
            formals.add(name)
    return formals


def _strip_formal_reassign_blocks(fn_text: str, formals: set[str]) -> str:
    """Drop release of __tmp when reassigning a formal (caller owns old value)."""
    if not formals:
        return fn_text
    pat = re.compile(
        r"\{\s*"
        r"(OoStr|OoSList|OoIList)\s+__tmp\s*=\s*([A-Za-z_][A-Za-z0-9_]*)\s*;\s*"
        r"\2\s*=\s*([^;]+);\s*"
        r"oo_(?:str|slist|ilist)_release\s*\(\s*__tmp\s*\)\s*;\s*"
        r"\}",
        re.MULTILINE,
    )

    def repl(m: re.Match[str]) -> str:
        var = m.group(2)
        rhs = m.group(3).strip()
        if var in formals:
            return f"/* no release formal reassign {var} */ {var} = {rhs};"
        return m.group(0)

    return pat.sub(repl, fn_text)


def strip_formal_param_releases(t: str) -> str:
    """Remove oo_*_release(formal) and formal-reassign __tmp releases.

    Params are caller-owned. Keeps body-local and non-formal reassign releases.
    """
    header_pattern = re.compile(
        r"^[ \t]*(?:static\s+)?(?:inline\s+)?"
        r"(?:void|int|long\s+long|double|float|bool|OoStr|OoSList|OoIList|OoResS|OoResV|Token)\s+"
        r"([A-Za-z0-9_]+)\s*\(([^)]*)\)\s*\{",
        re.MULTILINE,
    )
    out: list[str] = []
    pos = 0
    for m in header_pattern.finditer(t):
        start_fn = m.start()
        formals = _parse_formals(m.group(2).strip())
        curr = _scan_c_braces(t, m.end())
        if start_fn > pos:
            out.append(t[pos:start_fn])
        fn_text = t[start_fn:curr]
        if formals:
            fn_text = _strip_formal_reassign_blocks(fn_text, formals)

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
