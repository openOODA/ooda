"""Seed softener: retain after OoStr/list alias binds (let x = y)."""
from __future__ import annotations
import re

_ALIAS = re.compile(
    r"^([ \t]*)(OoStr|OoSList|OoIList)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*"
    r"([A-Za-z_][A-Za-z0-9_]*)\s*;[ \t]*$",
    re.MULTILINE,
)

_RETAIN = {
    "OoStr": "oo_str_retain",
    "OoSList": "oo_slist_retain",
    "OoIList": "oo_ilist_retain",
}


def inject_alias_retains(t: str) -> str:
    """After `OoT x = y;` (y bare ident), emit retain(x) once.

    Seed often copies ARC values without retain; free then double-releases.
    """
    if "alias retain softener" in t:
        return t

    def repl(m: re.Match[str]) -> str:
        indent, ty, name, rhs = m.group(1), m.group(2), m.group(3), m.group(4)
        if name.startswith("__tmp") or name == rhs:
            return m.group(0)
        ret = _RETAIN[ty]
        return f"{m.group(0)}\n{indent}{ret}({name}); /* alias retain softener */"

    return _ALIAS.sub(repl, t)
