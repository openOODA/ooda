#!/usr/bin/env python3
"""Parsing utilities for openOODA test scanning."""
from __future__ import annotations

def skip_ws(text: str, j: int, n: int) -> int:
    while j < n and text[j] in " \t\r\n":
        j += 1
    return j

def match_kw(text: str, j: int, n: int, kw: str) -> int:
    if text.startswith(kw, j):
        end = j + len(kw)
        if end >= n or not (text[end].isalnum() or text[end] == "_"):
            return end
    return -1

def skip_balanced(text: str, j: int, n: int, open_c: str = "{", close_c: str = "}") -> int:
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

def skip_paren_group(text: str, j: int, n: int) -> int:
    return skip_balanced(text, j, n, "(", ")")

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
