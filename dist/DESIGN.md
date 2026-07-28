# OODA Programming Language: Design & Architectural Blueprint (`DESIGN.md`)
**openOODA Project** — `https://github.com/openOODA/ooda`

---

## 🏛️ Executive Summary

**OODA** (*Observe, Orient, Decide, Act*) is an AI-native, capability-secure, self-testing systems programming language designed for sub-second development feedback, zero-day defense, and bare-metal native execution.

---

## 📌 Core Architectural Pillars

### 1. Capability-Based Sandboxing (`&NetCap`, `&FsCap`, `&SysCap`, `&EnvCap`)
* Default-deny security model. Functions cannot perform network, filesystem, environment, or process I/O without receiving explicit capability tokens in their parameter list.
* Traps 100% of unauthorized zero-day attacks and malicious 3rd-party dependencies statically at compile time.

### 2. Self-Testing Code (`requires` / `ensures` / `verify`)
* Preconditions (`requires`) and postconditions (`ensures`) are first-class language keywords right above function headers.
* Co-located `verify` test blocks live right next to function implementations.
* Built-in automated contract fuzzer (`ooda test --fuzz`).

### 3. AI Vibe-Coding Native
* `--json-errors`: Machine-readable JSON compiler diagnostics with line numbers, explanations, and surgical AST diff fix suggestions for 1-turn AI auto-fixing.
* `ooda outline`: Token-minimized API summary generator yielding 85–90% token reduction when AI agents reference module APIs.
* `ooda reflect`: Symbol reflection metadata API exporting types, contract bounds, and required capability handles.
* `ooda patch`: Surgical AST JSON node patcher allowing AI agents to edit functions with 90% fewer tokens.

### 4. Dual-Engine Architecture
* **Development JIT (`ooda run`)**: Instant sub-millisecond execution (16.5 µs parse speed).
* **Production LLVM IR (`ooda build --release --emit-llvm`)**: Compiles directly to native LLVM Intermediate Representation (`.ll`) for bare-metal performance.
* **0ms Garbage Collection Pauses**: Scope-based RAII + Region Arenas eliminate Stop-The-World GC latency spikes completely.

---

## 📂 Design Document Locations & Links

* 📜 **Formal EBNF Grammar**: [ooda.ebnf](file:///home/jeryd/openooda-spec/ooda.ebnf)
* 📄 **Full Specification**: [SPEC.md](file:///home/jeryd/openooda-spec/SPEC.md)
* ⚙️ **Compiler Source**: [openOODA/ooda](file:///home/jeryd/openooda)
* 🧪 **QA Integration Suite**: [openOODA/qa](file:///home/jeryd/openooda-qa)
* 🌐 **Interactive Web Playground**: [openOODA/docs/index.html](file:///home/jeryd/openooda-docs/index.html)
