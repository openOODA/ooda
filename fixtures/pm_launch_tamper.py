# Tamper lock/CAS so ooda launch can refuse on the shipped path.
# usage: python3 fixtures/pm_launch_tamper.py sha|caps
import pathlib
import sys

kind = sys.argv[1]
lock = pathlib.Path(".ooda_modules/lock")
lines = lock.read_text().splitlines()
out = []
for ln in lines:
    if ln.startswith("ingar@"):
        parts = ln.split("#")
        sha = parts[1]
        if kind == "sha":
            cas = pathlib.Path(".ooda_modules/cas") / sha
            hit = None
            for p in cas.iterdir():
                if p.name.startswith("ingest_"):
                    continue
                hit = p
                break
            if hit is None:
                raise SystemExit("no payload in " + str(cas))
            hit.write_bytes(hit.read_bytes() + b"X")
        if kind == "caps":
            ln = parts[0] + "#" + sha + "#Net"
    out.append(ln)
if kind == "caps":
    lock.write_text("\n".join(out) + "\n")
