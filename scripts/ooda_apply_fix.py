#!/usr/bin/env python3
"""Product ooda fix dispatcher: multi-pass E_CAP + E_TC + E_HITL (path A).

  ooda_apply_fix.py <file.oo> [--dry-run]

Path A: repeatedly apply supported classes until check is clean of
E_CAP / E_TC undefined-var / E_HITL pause, or max rounds.
Residual: other codes (E_PARSE rewrite, E_SECRET, …), free-form suggested_fix.
"""
from __future__ import annotations

import json, os, subprocess, sys
from pathlib import Path

MAX_ROUNDS = 8

def die(msg: str, code: int = 1) -> None:
    print(f"ERR\tfix\t{msg}", file=sys.stderr)
    raise SystemExit(code)

def resolve_oodac() -> str:
    for c in (
        os.environ.get("OODAC_BIN", ""),
        str(Path(__file__).resolve().parent.parent / "oodac" / "oodac"),
        "./oodac/oodac",
    ):
        if c and Path(c).is_file() and os.access(c, os.X_OK):
            return c
    die("no oodac", 2)

def load_codes(oodac: str, path: str) -> tuple[list[str], int, str]:
    p = subprocess.run(
        [oodac, "check", path, "--json-errors"],
        capture_output=True, text=True,
    )
    raw = (p.stdout or "") + "\n" + (p.stderr or "")
    lines = [ln for ln in raw.splitlines() if ln.strip().startswith("[")]
    codes: list[str] = []
    if lines:
        try:
            v = json.loads(lines[-1])
            codes = [d.get("code") for d in v if isinstance(d, dict)]
        except json.JSONDecodeError:
            die("invalid json-errors")
    return codes, p.returncode, raw

def main() -> None:
    if len(sys.argv) < 2:
        die("usage: ooda_apply_fix.py <file.oo> [--dry-run]", 2)
    path = sys.argv[1]
    extra = sys.argv[2:]
    dry = "--dry-run" in extra
    root = Path(__file__).resolve().parent
    oodac = resolve_oodac()

    applied = 0
    for rnd in range(MAX_ROUNDS):
        codes, rc, _ = load_codes(oodac, path)
        # Some diags (E_HITL) may still exit 0 after printing JSON; trust codes first.
        has_fixable = any(c in codes for c in ("E_CAP", "E_TC", "E_HITL"))
        if rc == 0 and not has_fixable:
            if applied == 0:
                die("check passed (nothing to fix)")
            print(f"OK\tfix\tmulti-pass clean after {applied} apply step(s)")
            raise SystemExit(0)

        step = None
        if "E_CAP" in codes:
            step = "ecap"
            script = root / "ooda_apply_ecap_fix.py"
        elif "E_TC" in codes:
            step = "etc"
            script = root / "ooda_apply_etc_fix.py"
        elif "E_HITL" in codes:
            step = "ehitl"
            script = root / "ooda_apply_ehitl_fix.py"
        else:
            if applied == 0:
                die(
                    "no supported diagnostic class "
                    "(want E_CAP, E_TC undefined-var, or E_HITL)"
                )
            die(
                f"multi-pass stopped after {applied} step(s); remaining codes={codes} "
                "(other codes residual — not multi-code product)"
            )

        r = subprocess.run([sys.executable, str(script), path, *extra])
        if r.returncode != 0:
            if applied == 0:
                raise SystemExit(r.returncode)
            die(f"apply {step} failed on round {rnd + 1} after {applied} prior step(s)")
        applied += 1
        if dry:
            # dry-run scripts print to stdout; one pass only
            print(f"OK\tfix\tdry-run single {step} step (multi-pass skipped in dry-run)")
            raise SystemExit(0)

    die(f"multi-pass hit max rounds ({MAX_ROUNDS}) after {applied} steps")

if __name__ == "__main__":
    main()
