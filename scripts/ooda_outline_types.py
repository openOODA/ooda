#!/usr/bin/env python3
"""Shared outline types and regexes."""
from __future__ import annotations

import re
from dataclasses import dataclass, field

CAP_RE = re.compile(r"&?(NetCap|FsCap|SysCap|EnvCap)\b")
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


@dataclass
class Param:
    name: str
    type_str: str  # includes leading & when present


@dataclass
class FnMeta:
    name: str
    is_pub: bool
    params: list[Param] = field(default_factory=list)
    ret: str = ""
    requires: list[str] = field(default_factory=list)
    ensures: list[str] = field(default_factory=list)

    def caps(self) -> list[str]:
        out: list[str] = []
        for p in self.params:
            m = CAP_RE.search(p.type_str)
            if m:
                c = m.group(1)
                if c not in out:
                    out.append(c)
        return out


@dataclass
class VerifyMeta:
    name: str


