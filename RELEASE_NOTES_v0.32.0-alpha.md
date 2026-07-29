## v0.32.0-alpha — Type fail-closed, CHS parity, leaner AI fixes

### Top-5 this rotation

1. **Contracts/types:** undefined free functions and unknown methods fail closed (no soft `Ty::Unknown` success). Sealed I/O typed as `Result`. Unit tests for undefined fn + fetch Result.
2. **Capabilities/QA:** real FsCap sandbox test; security pens stay traps; precondition trap phase; real while-loop stress.
3. **Dual engine:** `scripts/chs_semantic_parity.sh` — `ooda run` vs `ooda build --target c` digests for while/int/chs_hello; wired into QA Phase 7c2.
4. **AI diagnostics:** cap-violation `suggested_fix.diff` names the offending function + cap kind + effect; golden asserts it.
5. **E-M / ship:** portable `rfc_auditor.py`; bench suite version from Cargo; pin lock **v0.32.0-alpha**.

### Not claimed
Full self-host / zero `.rs`, LSP, pkg install, full LLVM List/String, Boyd T/D instrumentation.
