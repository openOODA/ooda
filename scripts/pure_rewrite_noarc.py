#!/usr/bin/env python3
"""PURE_NO_ARC=1 helper for oodac_pure_rewrite."""
from __future__ import annotations
import re

def strip_arc_calls_string_safe(t: str) -> str:
    """Remove oo_*_retain/release statements outside string lits.
    Seed-era ARC emission currently heap-corrupts self-hosted oodac;
    PURE_NO_ARC=1 uses this path so product backends can still ship."""
    out: list[str] = []
    i, n = 0, len(t)
    in_str = in_chr = escape = False
    while i < n:
        ch = t[i]
        if escape:
            out.append(ch)
            escape = False
            i += 1
            continue
        if in_str:
            out.append(ch)
            if ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
            i += 1
            continue
        if in_chr:
            out.append(ch)
            if ch == "\\":
                escape = True
            elif ch == "'":
                in_chr = False
            i += 1
            continue
        if ch == '"':
            in_str = True
            out.append(ch)
            i += 1
            continue
        if ch == "'":
            in_chr = True
            out.append(ch)
            i += 1
            continue
        if t.startswith("oo_", i) and (
            t.startswith("oo_str_release", i)
            or t.startswith("oo_str_retain", i)
            or t.startswith("oo_slist_release", i)
            or t.startswith("oo_slist_retain", i)
            or t.startswith("oo_ilist_release", i)
            or t.startswith("oo_ilist_retain", i)
        ):
            j = i
            while j < n and t[j] != "(":
                j += 1
            if j >= n:
                out.append(ch)
                i += 1
                continue
            depth, k = 1, j + 1
            while k < n and depth:
                if t[k] == '"':
                    k += 1
                    while k < n:
                        if t[k] == "\\":
                            k += 2
                            continue
                        if t[k] == '"':
                            k += 1
                            break
                        k += 1
                    continue
                if t[k] == "(":
                    depth += 1
                elif t[k] == ")":
                    depth -= 1
                k += 1
            while k < n and t[k] in " \t":
                k += 1
            if k < n and t[k] == ";":
                k += 1
            out.append("/*noarc*/;")
            i = k
            continue
        out.append(ch)
        i += 1
    s = "".join(out)
    s = re.sub(
        r"\{\s*(OoStr|OoSList|OoIList)\s+__tmp\s*=\s*([A-Za-z0-9_]+)\s*;\s*\2\s*=\s*([^;]+);\s*/\*noarc\*/;\s*\}",
        r"{ \2 = \3; }",
        s,
    )
    return s



def apply_pure_no_arc(path: str, t: str) -> None:
    t = strip_arc_calls_string_safe(t)
    decls = (
        "void oo_str_retain(OoStr); void oo_str_release(OoStr);\n"
        "void oo_slist_retain(OoSList); void oo_slist_release(OoSList);\n"
        "void oo_ilist_retain(OoIList); void oo_ilist_release(OoIList);\n"
        "long long oo_cap_grant_fs(void); long long oo_cap_grant_sys(void);\n"
        "long long oo_cap_grant_env(void); long long oo_cap_grant_net(void);\n"
        "void oo_cap_require(long long,long long,const char*);\n"
        "OoResS oo_sys_exec(long long,int,OoStr*); OoResS oo_sys_exec1(long long,OoStr);\n"
        "OoResS oo_fetch(long long,OoStr);\n"
    )
    if "void oo_str_retain" not in t[:8000]:
        needle = "typedef struct { int ok; OoStr err; } OoResV;"
        t = t.replace(needle, needle + "\n" + decls, 1) if needle in t else decls + t
    open(path, "w", encoding="utf-8").write(t)
    print("pure_rewrite: PURE_NO_ARC=1 (ARC stripped)", flush=True)
