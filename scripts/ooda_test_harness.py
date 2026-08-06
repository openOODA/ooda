#!/usr/bin/env python3
"""Lower .oo verify/assert_eq! blocks into a Backend-C harness main.

Used by scripts/ooda_test_verify.sh. Exit 0 with empty harness path content
means check-only (no verify). Residual: only assert_eq!; contracts stripped.
"""
from __future__ import annotations

import os
import re
import sys


def main() -> int:
    src_path = os.environ["OODA_TEST_SRC"]
    out_path = os.environ["OODA_TEST_HARNESS"]
    raw = open(src_path, "r", encoding="utf-8").read()

    # Strip requires/ensures (native emit does not lower contracts)
    lines = []
    for line in raw.splitlines(keepends=True):
        s = line.lstrip()
        if s.startswith("requires ") or s.startswith("ensures "):
            continue
        lines.append(line)
    text = "".join(lines)
    n = len(text)

    def skip_ws(j: int) -> int:
        while j < n and text[j] in " \t\r\n":
            j += 1
        return j

    def match_kw(j: int, kw: str) -> int:
        if text.startswith(kw, j):
            end = j + len(kw)
            if end >= n or not (text[end].isalnum() or text[end] == "_"):
                return end
        return -1

    def skip_balanced(j: int, open_c: str = "{", close_c: str = "}") -> int:
        if j >= n or text[j] != open_c:
            raise ValueError(f"expected {open_c} at {j}")
        depth = 0
        in_str = False
        esc = False
        while j < n:
            c = text[j]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == '"':
                    in_str = False
                j += 1
                continue
            if c == '"':
                in_str = True
                j += 1
                continue
            if c == open_c:
                depth += 1
            elif c == close_c:
                depth -= 1
                j += 1
                if depth == 0:
                    return j
                continue
            j += 1
        raise ValueError("unbalanced braces")

    def skip_paren_group(j: int) -> int:
        return skip_balanced(j, "(", ")")

    def parse_assert_eq_args(inner: str) -> tuple[str, str]:
        depth = 0
        in_str = False
        esc = False
        for k, c in enumerate(inner):
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == '"':
                    in_str = False
                continue
            if c == '"':
                in_str = True
                continue
            if c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
            elif c == "," and depth == 0:
                lhs = inner[:k].strip()
                rhs = inner[k + 1 :].strip()
                if not lhs or not rhs:
                    raise ValueError("empty assert_eq arg")
                return lhs, rhs
        raise ValueError("assert_eq needs two args")

    def bal(s: str, j0: int, open_c: str = "(", close_c: str = ")") -> int:
        j = j0
        depth = 0
        in_str = False
        esc = False
        ln = len(s)
        while j < ln:
            c = s[j]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == '"':
                    in_str = False
                j += 1
                continue
            if c == '"':
                in_str = True
                j += 1
                continue
            if c == open_c:
                depth += 1
            elif c == close_c:
                depth -= 1
                j += 1
                if depth == 0:
                    return j
                continue
            j += 1
        raise ValueError("unbalanced")

    asserts: list[tuple[str, str, str, int]] = []
    fns: list[str] = []
    verify_count = 0
    i = 0
    while i < n:
        i = skip_ws(i)
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
        j = match_kw(i, "pub")
        if j >= 0:
            i = skip_ws(j)

        j = match_kw(i, "fn")
        if j >= 0:
            i = skip_ws(j)
            m = re.match(r"[A-Za-z_][A-Za-z0-9_]*", text[i:])
            if not m:
                raise ValueError("fn without name")
            fname = m.group(0)
            i += len(fname)
            i = skip_ws(i)
            if i >= n or text[i] != "(":
                raise ValueError(f"fn {fname}: expected (")
            i = skip_paren_group(i)
            i = skip_ws(i)
            if text.startswith("->", i):
                i = skip_ws(i + 2)
                while i < n and text[i] != "{":
                    i += 1
            i = skip_ws(i)
            if i >= n or text[i] != "{":
                raise ValueError(f"fn {fname}: expected body {{")
            i = skip_balanced(i, "{", "}")
            chunk = text[start_item:i]
            if fname != "main":
                fns.append(chunk.rstrip() + "\n")
            continue

        i = start_item
        j = match_kw(i, "verify")
        if j >= 0:
            i = skip_ws(j)
            m = re.match(r"[A-Za-z_][A-Za-z0-9_]*", text[i:])
            if not m:
                raise ValueError("verify without name")
            vname = m.group(0)
            i += len(vname)
            i = skip_ws(i)
            if i >= n or text[i] != "{":
                raise ValueError(f"verify {vname}: expected {{")
            body_start = i + 1
            i = skip_balanced(i, "{", "}")
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
                if m_eq:
                    kind = "eq"
                    pos = pos + m_eq.end() - 1
                elif m_ne:
                    kind = "ne"
                    pos = pos + m_ne.end() - 1
                elif m_as:
                    kind = "assert"
                    pos = pos + m_as.end() - 1
                else:
                    snippet = b[pos : pos + 48].replace("\n", " ")
                    print(
                        f"ERR\ttest\tverify {vname}: only assert_eq!/assert_ne!/assert! ({snippet!r})",
                        file=sys.stderr,
                    )
                    return 1
                end = bal(b, pos, "(", ")")
                inner = b[pos + 1 : end - 1]
                aidx += 1
                if kind == "assert":
                    expr = inner.strip()
                    if not expr:
                        print(f"ERR\ttest\tverify {vname}: empty assert!", file=sys.stderr)
                        return 1
                    asserts.append((kind, expr, "", vname, aidx))
                else:
                    try:
                        lhs, rhs = parse_assert_eq_args(inner)
                    except ValueError as e:
                        print(f"ERR\ttest\tverify {vname}: {e}", file=sys.stderr)
                        return 1
                    asserts.append((kind, lhs, rhs, vname, aidx))
                pos = end
                while pos < blen and b[pos] in " \t\r\n":
                    pos += 1
                if pos < blen and b[pos] == ";":
                    pos += 1
            continue

        j = match_kw(i, "type")
        if j < 0:
            j = match_kw(i, "import")
        if j >= 0:
            while i < n and text[i] != ";":
                i += 1
            if i < n:
                i += 1
            continue

        snippet = text[i : i + 40].replace("\n", " ")
        print(f"ERR\ttest\tunsupported top-level near: {snippet!r}", file=sys.stderr)
        return 1

    if verify_count == 0:
        print("OK\ttest\tcheck-only (no verify blocks)")
        open(out_path, "w", encoding="utf-8").write("")
        return 0

    if not asserts:
        print("ERR\ttest\tverify blocks present but no asserts", file=sys.stderr)
        return 1

    parts: list[str] = ["// generated by ooda_test_harness.py — do not edit\n"]
    for f in fns:
        parts.append(f if f.endswith("\n") else f + "\n")
    parts.append("\npub fn main() {\n")
    for kind, a, b, vname, aidx in asserts:
        if kind == "eq":
            parts.append(f"    if {a} != {b} {{\n")
            parts.append(f'        println("FAIL assert_eq {vname}#{aidx}");\n')
        elif kind == "ne":
            parts.append(f"    if {a} == {b} {{\n")
            parts.append(f'        println("FAIL assert_ne {vname}#{aidx}");\n')
        else:
            parts.append(f"    if !({a}) {{\n")
            parts.append(f'        println("FAIL assert {vname}#{aidx}");\n')
        parts.append("        process_exit(1);\n")
        parts.append("    }\n")
    parts.append(f'    println("OK verify ({len(asserts)} asserts)");\n')
    parts.append("}\n")
    open(out_path, "w", encoding="utf-8").write("".join(parts))
    print(
        f"OK\ttest\tharness {len(asserts)} asserts from {verify_count} verify",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:
        print(f"ERR\ttest\tharness: {e}", file=sys.stderr)
        sys.exit(1)
