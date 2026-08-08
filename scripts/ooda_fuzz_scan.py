#!/usr/bin/env python3
"""Scan .oo functions and contracts for fuzzer harness generation."""
from __future__ import annotations
import re
from ooda_fuzz_emit import emit_fuzz_harness

def collect_fuzz_targets(text: str) -> tuple[list[str], list[dict], bool]:
    lines = text.splitlines()
    n = len(lines)
    i = 0
    fns = []
    fuzz_targets = []
    has_contracts = False

    while i < n:
        line = lines[i].strip()
        if not line or line.startswith("//"):
            i += 1
            continue

        if line.startswith("verify ") or line == "verify":
            depth = 0
            while i < n:
                if '{' in lines[i]: depth += lines[i].count('{')
                if '}' in lines[i]: depth -= lines[i].count('}')
                i += 1
                if depth <= 0: break
            continue

        m = re.match(r'^(pub\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)(?:\s*->\s*([^{\n]+))?', line)
        if m:
            is_pub = bool(m.group(1))
            fname = m.group(2)
            params_raw = m.group(3).strip()
            ret_type = m.group(4).strip() if m.group(4) else "()"

            params = []
            if params_raw:
                p_parts = [p.strip() for p in params_raw.split(',') if p.strip()]
                for p in p_parts:
                    if ':' in p:
                        pname, ptype = p.split(':', 1)
                        params.append((pname.strip(), ptype.strip()))

            requires_clauses = []
            ensures_clauses = []
            body_code = []

            depth = 0
            started = False
            while i < n:
                curr_line = lines[i]
                s_curr = curr_line.strip()

                if s_curr.startswith("requires "):
                    requires_clauses.append(s_curr[len("requires "):].strip())
                    has_contracts = True
                elif s_curr.startswith("ensures "):
                    ensures_clauses.append(s_curr[len("ensures "):].strip())
                    has_contracts = True
                else:
                    if not started:
                        if '{' in curr_line:
                            started = True
                            depth += curr_line.count('{')
                            if '}' in curr_line: depth -= curr_line.count('}')
                            idx_brace = curr_line.find('{')
                            after = curr_line[idx_brace+1:].rstrip()
                            if '}' in after:
                                idx_close = after.find('}')
                                after = after[:idx_close]
                            if after.strip(): body_code.append(after)
                    else:
                        if '{' in curr_line: depth += curr_line.count('{')
                        if '}' in curr_line: depth -= curr_line.count('}')
                        if depth <= 0:
                            idx_close = curr_line.find('}')
                            if idx_close > 0:
                                before = curr_line[:idx_close]
                                if before.strip(): body_code.append(before)
                            i += 1
                            break
                        else:
                            body_code.append(curr_line)

                i += 1
                if started and depth <= 0:
                    break

            clean_header = f"{'pub ' if is_pub else ''}fn {fname}({params_raw})"
            if ret_type != "()":
                clean_header += f" -> {ret_type}"

            cleaned_code = clean_header + " {\n" + "\n".join(body_code) + "\n}\n"
            if fname != "main":
                fns.append(cleaned_code)
                fuzz_targets.append({
                    'fname': fname,
                    'params': params,
                    'return_type': ret_type,
                    'requires': requires_clauses,
                    'ensures': ensures_clauses
                })
            continue
        else:
            if not line.startswith("requires ") and not line.startswith("ensures "):
                fns.append(lines[i])
            i += 1

    return fns, fuzz_targets, has_contracts
