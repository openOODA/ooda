#!/usr/bin/env python3
"""Scan/parse .oo text into outline items (no execute)."""
from __future__ import annotations

from ooda_outline_types import CAP_RE, IDENT, FnMeta, Param, VerifyMeta

def _skip_line_comment(s: str, i: int) -> int:
    while i < len(s) and s[i] != "\n":
        i += 1
    return i

def _skip_block_comment(s: str, i: int) -> int:
    end = s.find("*/", i + 2)
    return len(s) if end < 0 else end + 2

def _skip_string(s: str, i: int) -> int:
    # s[i] is opening "
    i += 1
    while i < len(s):
        c = s[i]
        if c == "\\":
            i += 2
            continue
        if c == '"':
            return i + 1
        i += 1
    return i

def _match_kw(s: str, i: int, kw: str) -> int:
    if not s.startswith(kw, i):
        return -1
    end = i + len(kw)
    if end < len(s) and (s[end].isalnum() or s[end] == "_"):
        return -1
    return end

def _skip_ws_nl(s: str, i: int) -> int:
    n = len(s)
    while i < n:
        c = s[i]
        if c in " \t\r\n":
            i += 1
            continue
        if s.startswith("//", i):
            i = _skip_line_comment(s, i)
            continue
        if s.startswith("/*", i):
            i = _skip_block_comment(s, i)
            continue
        break
    return i

def _balanced(s: str, i: int, open_c: str, close_c: str) -> int:
    if i >= len(s) or s[i] != open_c:
        raise ValueError(f"expected {open_c}")
    depth = 0
    n = len(s)
    while i < n:
        c = s[i]
        if c == '"':
            i = _skip_string(s, i)
            continue
        if s.startswith("//", i):
            i = _skip_line_comment(s, i)
            continue
        if s.startswith("/*", i):
            i = _skip_block_comment(s, i)
            continue
        if c == open_c:
            depth += 1
        elif c == close_c:
            depth -= 1
            i += 1
            if depth == 0:
                return i
            continue
        i += 1
    raise ValueError("unbalanced")

def _parse_params(inner: str) -> list[Param]:
    inner = inner.strip()
    if not inner:
        return []
    parts: list[str] = []
    depth = 0
    start = 0
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
            parts.append(inner[start:k].strip())
            start = k + 1
    parts.append(inner[start:].strip())
    params: list[Param] = []
    for p in parts:
        if not p:
            continue
        if ":" not in p:
            params.append(Param(name=p, type_str=""))
            continue
        name, ty = p.split(":", 1)
        params.append(Param(name=name.strip(), type_str=ty.strip()))
    return params

def _parse_ret_and_contracts(s: str, i: int) -> tuple[str, list[str], list[str], int]:
    """After params ')': optional -> Type, then requires/ensures, then body '{'."""
    i = _skip_ws_nl(s, i)
    ret = ""
    if s.startswith("->", i):
        i = _skip_ws_nl(s, i + 2)
        start = i
        while i < len(s):
            c = s[i]
            if c in "{\n" or s.startswith("requires", i) or s.startswith("ensures", i):
                break
            if c == "/" and i + 1 < len(s) and s[i + 1] in "/*":
                break
            i += 1
        ret = s[start:i].strip()
        i = _skip_ws_nl(s, i)
    requires: list[str] = []
    ensures: list[str] = []
    while True:
        i = _skip_ws_nl(s, i)
        if i >= len(s):
            break
        if s[i] == "{":
            break
        j = _match_kw(s, i, "requires")
        if j >= 0:
            j = _skip_ws_nl(s, j)
            start = j
            while j < len(s) and s[j] not in "\n{":
                j += 1
            requires.append(s[start:j].strip())
            i = j
            continue
        j = _match_kw(s, i, "ensures")
        if j >= 0:
            j = _skip_ws_nl(s, j)
            start = j
            while j < len(s) and s[j] not in "\n{":
                j += 1
            ensures.append(s[start:j].strip())
            i = j
            continue
        # unexpected junk before body — stop
        break
    return ret, requires, ensures, i

def parse_module(text: str) -> list[FnMeta | VerifyMeta]:
    """Top-level scan: fn / pub fn / verify in source order. No execution."""
    s = text
    n = len(s)
    items: list[FnMeta | VerifyMeta] = []
    i = 0
    while i < n:
        i = _skip_ws_nl(s, i)
        if i >= n:
            break
        is_pub = False
        j = _match_kw(s, i, "pub")
        if j >= 0:
            is_pub = True
            j = _skip_ws_nl(s, j)
            k = _match_kw(s, j, "fn")
            if k < 0:
                i = j
                m = IDENT.match(s, i)
                i = m.end() if m else i + 1
                continue
            i = k
        else:
            k = _match_kw(s, i, "fn")
            if k >= 0:
                i = k
            else:
                k = _match_kw(s, i, "verify")
                if k >= 0:
                    k = _skip_ws_nl(s, k)
                    m = IDENT.match(s, k)
                    if not m:
                        i = k + 1
                        continue
                    vname = m.group(0)
                    k = _skip_ws_nl(s, m.end())
                    if k < n and s[k] == "{":
                        try:
                            k = _balanced(s, k, "{", "}")
                        except ValueError:
                            return items
                    items.append(VerifyMeta(name=vname))
                    i = k
                    continue
                if s[i] == "{":
                    try:
                        i = _balanced(s, i, "{", "}")
                    except ValueError:
                        break
                    continue
                m = IDENT.match(s, i)
                i = m.end() if m else i + 1
                continue

        i = _skip_ws_nl(s, i)
        m = IDENT.match(s, i)
        if not m:
            i += 1
            continue
        name = m.group(0)
        i = _skip_ws_nl(s, m.end())
        if i >= n or s[i] != "(":
            continue
        try:
            end_paren = _balanced(s, i, "(", ")")
        except ValueError:
            break
        params = _parse_params(s[i + 1 : end_paren - 1])
        ret, requires, ensures, i = _parse_ret_and_contracts(s, end_paren)
        items.append(
            FnMeta(
                name=name,
                is_pub=is_pub,
                params=params,
                ret=ret,
                requires=requires,
                ensures=ensures,
            )
        )
        i = _skip_ws_nl(s, i)
        if i < n and s[i] == "{":
            try:
                i = _balanced(s, i, "{", "}")
            except ValueError:
                break
    return items

