#!/usr/bin/env python3
"""Scan .oo verify/assert blocks for test harness (no execute)."""
from __future__ import annotations
import re
import sys
from ooda_fuzz_scan import collect_fuzz_targets
from ooda_fuzz_emit import emit_fuzz_harness
from ooda_test_parse_util import (
    skip_ws,
    match_kw,
    skip_balanced,
    skip_paren_group,
    parse_assert_eq_args,
    bal,
)

def collect_verifies(text: str) -> tuple[list[str], list[tuple], int]:
    n = len(text)
    asserts: list[tuple] = []
    fns: list[str] = []
    verify_count = 0
    i = 0
    while i < n:
        i = skip_ws(text, i, n)
        if i >= n:
            break
        if text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
            continue
        if text.startswith("/*", i):
            end = text.find("*/", i + 2)
            if end < 0:
                raise ValueError("unclosed block comment")
            i = end + 2
            continue
        start_item = i
        j = match_kw(text, i, n, "pub")
        if j >= 0:
            i = skip_ws(text, j, n)

        j = match_kw(text, i, n, "fn")
        if j >= 0:
            i = skip_ws(text, j, n)
            m = re.match(r"[A-Za-z_][A-Za-z0-9_]*", text[i:])
            if not m:
                raise ValueError("fn without name")
            fname = m.group(0)
            i += len(fname)
            i = skip_ws(text, i, n)
            if i >= n or text[i] != "(":
                raise ValueError(f"fn {fname}: expected (")
            i = skip_paren_group(text, i, n)
            i = skip_ws(text, i, n)
            if text.startswith("->", i):
                i = skip_ws(text, i + 2, n)
                while i < n and text[i] != "{":
                    i += 1
            i = skip_ws(text, i, n)
            if i >= n or text[i] != "{":
                raise ValueError(f"fn {fname}: expected body {{")
            i = skip_balanced(text, i, n, "{", "}")
            chunk = text[start_item:i]
            if fname != "main":
                fns.append(chunk.rstrip() + "\n")
            continue

        i = start_item
        j = match_kw(text, i, n, "verify")
        if j >= 0:
            i = skip_ws(text, j, n)
            m = re.match(r"[A-Za-z_][A-Za-z0-9_]*", text[i:])
            if not m:
                raise ValueError("verify without name")
            vname = m.group(0)
            i += len(vname)
            i = skip_ws(text, i, n)
            if i >= n or text[i] != "{":
                raise ValueError(f"verify {vname}: expected {{")
            body_start = i + 1
            i = skip_balanced(text, i, n, "{", "}")
            body = text[body_start : i - 1]
            verify_count += 1
            b = body
            pos = 0
            blen = len(b)
            aidx = 0
            while pos < blen:
                while pos < blen and b[pos] in " \t\r\n":
                    pos += 1
                if pos >= blen:
                    break
                if b.startswith("//", pos):
                    while pos < blen and b[pos] != "\n":
                        pos += 1
                    continue
                m_eq = re.match(r"assert_eq!\s*\(", b[pos:])
                m_ne = re.match(r"assert_ne!\s*\(", b[pos:])
                m_as = re.match(r"assert!\s*\(", b[pos:])
                m_let = re.match(r"let\s+", b[pos:])
                if m_eq: kind = "eq"; pos = pos + m_eq.end() - 1
                elif m_ne: kind = "ne"; pos = pos + m_ne.end() - 1
                elif m_as: kind = "assert"; pos = pos + m_as.end() - 1
                elif m_let:
                    end_semi = b.find(";", pos)
                    if end_semi != -1:
                        stmt = b[pos : end_semi + 1]; asserts.append(("stmt", stmt, "", vname, aidx)); pos = end_semi + 1; continue
                    else: raise ValueError(f"verify {vname}: unclosed let statement")
                else:
                    snippet = b[pos : pos + 48].replace("\n", " ")
                    raise ValueError(
                        f"verify {vname}: only assert_eq!/assert_ne!/assert! ({snippet!r})"
                    )
                end = bal(b, pos, "(", ")")
                inner = b[pos + 1 : end - 1]
                aidx += 1
                if kind == "assert":
                    expr = inner.strip()
                    if not expr:
                        raise ValueError(f"verify {vname}: empty assert!")
                    asserts.append((kind, expr, "", vname, aidx))
                else:
                    lhs, rhs = parse_assert_eq_args(inner)
                    asserts.append((kind, lhs, rhs, vname, aidx))
                pos = end
                while pos < blen and b[pos] in " \t\r\n":
                    pos += 1
                if pos < blen and b[pos] == ";":
                    pos += 1
            continue

        j = match_kw(text, i, n, "type")
        if j < 0: j = match_kw(text, i, n, "import")
        if j >= 0:
            start_i = i
            while i < n and text[i] != ";": i += 1
            if i < n: i += 1
            if match_kw(text, start_i, n, "import") >= 0: fns.append(text[start_i:i])
            continue

        snippet = text[i : i + 40].replace("\n", " ")
        raise ValueError(f"unsupported top-level near: {snippet!r}")

    return fns, asserts, verify_count

def emit_harness(fns: list[str], asserts: list[tuple]) -> str:
    parts: list[str] = ["// generated by ooda_test_harness.py — do not edit\n"]
    for f in fns: parts.append(f if f.endswith("\n") else f + "\n")
    parts.append("\npub fn main() {\n")
    cur_vname = None
    real_assert_count = 0
    for kind, a, b, vname, aidx in asserts:
        if vname != cur_vname:
            parts.append(f"    // verify {vname}\n")
            cur_vname = vname
        if kind == "stmt": parts.append(f"    {a}\n")
        elif kind == "eq": real_assert_count += 1; parts.append(f"    if {a} != {b} {{\n        println(\"FAIL assert_eq {vname}#{aidx}\");\n        process_exit(1);\n    }}\n")
        elif kind == "ne": real_assert_count += 1; parts.append(f"    if {a} == {b} {{\n        println(\"FAIL assert_ne {vname}#{aidx}\");\n        process_exit(1);\n    }}\n")
        else: real_assert_count += 1; parts.append(f"    if !({a}) {{\n        println(\"FAIL assert {vname}#{aidx}\");\n        process_exit(1);\n    }}\n")
    parts.append(f'    println("OK verify ({real_assert_count} asserts)");\n')
    parts.append("}\n")
    return "".join(parts)
