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

from ooda_test_scan import collect_verifies, emit_harness, collect_fuzz_targets, emit_fuzz_harness


def main() -> int:
    src_path = os.environ["OODA_TEST_SRC"]
    out_path = os.environ["OODA_TEST_HARNESS"]
    fuzz_mode = os.environ.get("OODA_TEST_FUZZ", "0") == "1"
    fuzz_iters = int(os.environ.get("OODA_TEST_FUZZ_ITERS", "100"))
    fuzz_seed = int(os.environ.get("OODA_TEST_FUZZ_SEED", "42"))
    fuzz_verbose = os.environ.get("OODA_TEST_FUZZ_VERBOSE", "0") == "1"

    raw = open(src_path, "r", encoding="utf-8").read()

    if fuzz_mode:
        fns, fuzz_targets, has_contracts = collect_fuzz_targets(raw)
        code = emit_fuzz_harness(fns, fuzz_targets, has_contracts, fuzz_iters, fuzz_seed, fuzz_verbose)
        open(out_path, "w", encoding="utf-8").write(code)
        return 0

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
