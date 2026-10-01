# ooda: Agent Engineering Standards (v1)

This repository houses the back-compat router for openOODA.
All work in this repository strictly defers to the organization standards in [`openOODA/AGENTS.md`](file:///home/ubermetroid/Projects/openOODA/openOODA/AGENTS.md).

---

## 1. Router Architecture & Invariants
- **Thin Forwarding**: The `ooda` binary executes no compilation or language tasks in-process. It resolves and spawns `cli`, `opm`, `lsp`, `mcp`, or `tui`.
- **Default Action**: Bare `ooda` (with no arguments) spawns `tui`.
- **Retired Subcommands**: Verbs that moved off the router (`bench`, `health`, `digest`, `sandbox`, `swarm`) print clear redirection messages and exit 2.

---

## 2. Invariants & Quality Standards
- **The Page Rule**: Every `.oo` page must be between 16 and 256 lines.
- **Directory Density**: At most 8 `.oo` pages per directory.
- **4-Element Academy Header**: Mandatory on every `.oo` page.
- **Double-Run Determinism**: All `qa/probe_*.oo` tests must pass in sequential fresh processes.

---

## 3. Local Verification Commands
```bash
cli build main.oo -o dist/ooda
cli qa
```
