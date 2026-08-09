#!/usr/bin/env python3
"""Rewrite seed-era all.c for process-local caps + chs_rt sys/fetch."""
from __future__ import annotations
import re
import sys
from pathlib import Path as _Path
sys.path.insert(0, str(_Path(__file__).resolve().parent))

def rewrite(fn: str, cap: str, s: str) -> str:
    out: list[str] = []
    i, key, n, in_str = 0, fn + "(", len(s), False
    while i < n:
        c = s[i]
        if in_str:
            out.append(c)
            if c == "\\" and i + 1 < n:
                out.append(s[i + 1])
                i += 2
                continue
            if c == '"': in_str = False
            i += 1
            continue
        if c == '"':
            in_str = True
            out.append(c)
            i += 1
            continue
        if s.startswith(key, i) and (i == 0 or not (s[i - 1].isalnum() or s[i - 1] == "_")):
            k = i + len(key)
            depth, start_a, comma = 1, k, False
            while k < n and depth:
                ch = s[k]
                if ch == '"':
                    k += 1
                    while k < n:
                        if s[k] == "\\":
                            k += 2
                            continue
                        if s[k] == '"':
                            k += 1
                            break
                        k += 1
                    continue
                if ch == "(": depth += 1
                elif ch == ")":
                    depth -= 1
                    if depth == 0: break
                elif ch == "," and depth == 1: comma = True
                k += 1
            args = s[start_a:k]
            a = args.strip()
            if (not comma) and a and not a.startswith("OoStr") and not a.startswith("long long") and not a.startswith("int "):
                out.append(f"{fn}({cap}, {args})")
            else:
                out.append(s[i : k + 1])
            i = k + 1
            continue
        out.append(c)
        i += 1
    return "".join(out)

def strip_static_fn(src: str, sig: str) -> str:
    """Remove a top-level static inline fn. Must not match sig text that
    appears inside string lits (c_emit_preamble println of seed-era helpers)."""
    # Only line-leading matches (optional indent) — not mid-string.
    m = re.search(r"(?m)^[ \t]*" + re.escape(sig), src)
    if not m:
        return src
    i = m.start()
    j = src.find("{", m.end() - 1)
    if j < 0:
        return src
    k = _scan_c_braces(src, j + 1)
    return src[:i] + "/* stripped seed inline; chs_rt provides */\n" + src[k:]

def _scan_c_braces(t: str, start: int) -> int:
    """Return index just past matching '}' for a body that starts at start (after '{').
    String/char-aware so oo string lits with '{' / '}' do not explode the scan
    (was duplicating half the TU into each early function — pure_build redef hell)."""
    brace_count = 1
    curr = start
    n = len(t)
    in_str = False
    in_chr = False
    escape = False
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


def fix_mismatched_releases(t: str) -> str:
    out_chunks = []
    pos = 0
    header_pattern = re.compile(
        r"^[ \t]*(?:static\s+)?(?:inline\s+)?(?:void|int|long\s+long|OoStr|OoSList|OoIList|OoResS|OoResV)\s+([A-Za-z0-9_]+)\s*\([^)]*\)\s*\{",
        re.MULTILINE,
    )
    rel_pattern = re.compile(
        r"\b(oo_slist_release|oo_ilist_release|oo_str_release|oo_slist_retain|oo_ilist_retain|oo_str_retain)\s*\(\s*([A-Za-z0-9_]+)\s*\)"
    )

    seen_funcs = set()

    for m in header_pattern.finditer(t):
        start_fn = m.start()
        func_name = m.group(1)

        curr = _scan_c_braces(t, m.end())

        if func_name in seen_funcs:
            if start_fn > pos: out_chunks.append(t[pos:start_fn])
            pos = curr
            continue

        seen_funcs.add(func_name)
        fn_text = t[start_fn:curr]

        if re.search(r"\b(i|li|fi)\s*=\s*", fn_text) and not re.search(r"\b(long long|int)\s+(i|li|fi)\b", fn_text):
            fn_text = re.sub(r"\{\n", "{\n  long long i = 0;\n", fn_text, count=1)

        slist_vars = set(re.findall(r"\bOoSList\s+([A-Za-z0-9_]+)", fn_text))
        ilist_vars = set(re.findall(r"\bOoIList\s+([A-Za-z0-9_]+)", fn_text))
        str_vars = set(re.findall(r"\bOoStr\s+([A-Za-z0-9_]+)", fn_text)) - (slist_vars | ilist_vars)

        def replace_rel(rm):
            func, var = rm.group(1), rm.group(2)
            if var in ("__tmp", "__ret_val"): return rm.group(0)
            if var in slist_vars:
                if func in ("oo_str_release", "oo_ilist_release"): return f"oo_slist_release({var})"
                if func in ("oo_str_retain", "oo_ilist_retain"): return f"oo_slist_retain({var})"
                return rm.group(0)
            elif var in ilist_vars:
                if func in ("oo_str_release", "oo_slist_release"): return f"oo_ilist_release({var})"
                if func in ("oo_str_retain", "oo_slist_retain"): return f"oo_ilist_retain({var})"
                return rm.group(0)
            elif var in str_vars:
                if func in ("oo_slist_release", "oo_ilist_release"): return f"oo_str_release({var})"
                if func in ("oo_slist_retain", "oo_ilist_retain"): return f"oo_str_retain({var})"
                return rm.group(0)
            return rm.group(0)

        fn_text = rel_pattern.sub(replace_rel, fn_text)
        if start_fn > pos: out_chunks.append(t[pos:start_fn])
        out_chunks.append(fn_text)
        pos = curr

    if pos < len(t): out_chunks.append(t[pos:])
    res = "".join(out_chunks)
    proto_lines, seen_protos = [], set()
    for line in res.splitlines(keepends=True):
        if line.endswith(";\n") or line.endswith(";"):
            pm = re.match(r"^[ \t]*(?:void|int|long\s+long|OoStr|OoSList|OoIList|OoResS|OoResV)\s+([A-Za-z0-9_]+)\s*\(", line)
            if pm:
                pname = pm.group(1)
                if pname in seen_protos: continue
                seen_protos.add(pname)
        proto_lines.append(line)
    return "".join(proto_lines)

def main() -> int:
    if len(sys.argv) != 2:
        print("usage: oodac_pure_rewrite.py <all.c>", file=sys.stderr)
        return 2
    path = sys.argv[1]
    t = open(path, encoding="utf-8", errors="replace").read()
    # PURE_NO_ARC=1: strip seed-emitted ARC (self-host residual until M2 closed)
    if __import__("os").environ.get("PURE_NO_ARC") == "1":
        from pure_rewrite_noarc import apply_pure_no_arc
        apply_pure_no_arc(path, t)
        return 0
    t = fix_mismatched_releases(t)
    from pure_rewrite_formals import strip_formal_param_releases
    t = strip_formal_param_releases(t)
    t = re.sub(r"(?<![A-Za-z0-9_])file_size\(", "oo_file_size(", t)
    for fn, cap in [
        ("oo_read_file", "oo_cap_grant_fs()"), ("oo_write_file", "oo_cap_grant_fs()"),
        ("oo_path_exists", "oo_cap_grant_fs()"), ("oo_file_size", "oo_cap_grant_fs()"),
        ("oo_env_get", "oo_cap_grant_env()"), ("oo_sys_exec1", "oo_cap_grant_sys()"),
        ("oo_sys_exec", "oo_cap_grant_sys()"),
    ]:
        t = rewrite(fn, cap, t)
    for old_c, new_c in [
        ("int fs = 0; int sys = 0;", "long long fs = oo_cap_grant_fs(); long long sys = oo_cap_grant_sys(); long long env = oo_cap_grant_env(); long long net = oo_cap_grant_net();"),
        ("long long fs = 0; long long sys = 0;", "long long fs = oo_cap_grant_fs(); long long sys = oo_cap_grant_sys(); long long env = oo_cap_grant_env(); long long net = oo_cap_grant_net();"),
        ("long long fs = OO_CAP_FS; long long sys = OO_CAP_SYS; long long env = OO_CAP_ENV; long long net = OO_CAP_NET;", "long long fs = oo_cap_grant_fs(); long long sys = oo_cap_grant_sys(); long long env = oo_cap_grant_env(); long long net = oo_cap_grant_net();"),
        ("long long fs = OO_CAP_FS; long long sys = OO_CAP_SYS; long long env = OO_CAP_ENV;", "long long fs = oo_cap_grant_fs(); long long sys = oo_cap_grant_sys(); long long env = oo_cap_grant_env(); long long net = oo_cap_grant_net();"),
    ]:
        t = t.replace(old_c, new_c)
    t = strip_static_fn(t, "static inline OoResS oo_sys_exec1(long long cap, OoStr cmd)")
    t = strip_static_fn(t, "static inline OoResS oo_fetch(long long cap, OoStr url)")
    t = strip_static_fn(t, "static inline void oo_cap_require(long long got, long long want, const char *op)")
    decls = (
        "long long oo_cap_grant_fs(void); long long oo_cap_grant_sys(void);\n"
        "long long oo_cap_grant_env(void); long long oo_cap_grant_net(void);\n"
        "void oo_cap_require(long long,long long,const char*);\n"
        "OoResS oo_sys_exec(long long,int,OoStr*); OoResS oo_sys_exec1(long long,OoStr);\n"
        "OoResS oo_fetch(long long,OoStr);\n"
        "void oo_ilist_free(OoIList); void oo_slist_free(OoSList);\n"
        "void oo_slist_retain(OoSList); void oo_slist_release(OoSList);\n"
        "void oo_str_retain(OoStr); void oo_str_release(OoStr);\n"
        "void oo_ilist_retain(OoIList); void oo_ilist_release(OoIList);\n"
    )
    m = re.search(r"typedef struct \{ int ok; OoStr err; \} OoResV;", t)
    if m: t = t[: m.end()] + "\n" + decls + t[m.end() :]
    else: t = decls + t
    for a, b in [
        ("OoResS oo_read_file(OoStr);", "OoResS oo_read_file(long long,OoStr);"),
        ("OoResV oo_write_file(OoStr,OoStr);", "OoResV oo_write_file(long long,OoStr,OoStr);"),
        ("int oo_path_exists(OoStr);", "int oo_path_exists(long long,OoStr);"),
        ("long long oo_file_size(OoStr);", "long long oo_file_size(long long,OoStr);"),
        ("OoResS oo_env_get(OoStr);", "OoResS oo_env_get(long long,OoStr);"),
        ("static inline OoResS oo_sys_exec1(OoStr cmd)", "OoResS oo_sys_exec1(long long cap, OoStr cmd)"),
    ]:
        t = t.replace(a, b)
    open(path, "w", encoding="utf-8").write(t)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
