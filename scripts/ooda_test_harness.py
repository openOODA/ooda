#!/usr/bin/env python3
"""Lower .oo verify/assert_eq! blocks into a Backend-C harness main.

Used by scripts/ooda_test_verify.sh. Exit 0 with empty harness = check-only.
Residual: only assert_eq!/assert_ne!/assert!; contracts stripped.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

_sysdir = Path(__file__).resolve().parent
if str(_sysdir) not in sys.path:
    sys.path.insert(0, str(_sysdir))

from ooda_test_scan import collect_verifies, emit_harness


def main() -> int:
    src_path = os.environ["OODA_TEST_SRC"]
    out_path = os.environ["OODA_TEST_HARNESS"]
    raw = open(src_path, "r", encoding="utf-8").read()
    lines = []
    for line in raw.splitlines(keepends=True):
        s = line.lstrip()
        if s.startswith("requires ") or s.startswith("ensures "):
            continue
        lines.append(line)
    text = "".join(lines)
    try:
        fns, asserts, verify_count = collect_verifies(text)
    except ValueError as e:
        print(f"ERR	test	harness: {e}", file=sys.stderr)
        return 1
    if verify_count == 0:
        print("OK	test	check-only (no verify blocks)")
        open(out_path, "w", encoding="utf-8").write("")
        return 0
    if not asserts:
        print("ERR	test	verify blocks present but no asserts", file=sys.stderr)
        return 1
    code = emit_harness(fns, asserts)
    open(out_path, "w", encoding="utf-8").write(code)
    print(
        f"OK	test	harness {len(asserts)} asserts from {verify_count} verify",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:
        print(f"ERR	test	harness: {e}", file=sys.stderr)
        sys.exit(1)
