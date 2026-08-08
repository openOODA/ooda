# OODA Programming Language: Design & Architectural Blueprint (`DESIGN.md`)
**openOODA Project** — `https://github.com/openOODA/ooda`

---

## 🏛️ Executive Summary

**OODA** (*Observe, Orient, Decide, Act*) is an AI-native, capability-secure, self-testing systems programming language designed for sub-second development feedback, zero-day defense, and bare-metal native execution. It is built to seamlessly scale from embedded hardware to a global, verifiable compute network.

---

## 1. The openOODA Language (Core Syntax & Philosophy)

*   **The Philosophy of Speed (The OODA Loop):** The language is fundamentally engineered around the *Observe, Orient, Decide, Act* cycle. By combining sub-millisecond compile times with rich JSON metadata, the language guarantees the tightest possible feedback loop between human intent, AI generation, and compiler validation.
*   **Mathematical Contracts:** Preconditions (`requires`) and postconditions (`ensures`) are first-class language keywords that mathematically bound function behavior, forming the foundation for local fuzzing and global verification.
*   **Data-Oriented Design (DOD) & Layout:** First-class support for Struct-of-Arrays (SoA) memory layouts to guarantee CPU cache-locality, and native zero-copy serialization for parsing network packets at zero CPU cost.
*   **First-Class AST Macros:** The ability to write standard openOODA functions that execute at compile-time to surgically rewrite the Abstract Syntax Tree (AST), providing the power of Rust macros with native readability.
*   **Compile-Time Type-State Machines:** Objects can transition through explicit lifecycles (e.g., `Unopened` -> `Opened` -> `Closed`). The compiler statically proves that `.read()` can never be called on an `Unopened` file, eliminating entire classes of logic bugs.

## 2. AI-Native Tooling (Vibe-Coding)

*   **Surgical AST Patching:** `--json-errors` emits machine-readable diagnostics with surgical AST diff-fix suggestions, allowing AI agents to auto-fix code in a single turn.
*   **Token-Minimized APIs:** `ooda outline` and `ooda reflect` export compressed symbol metadata, yielding an 85–90% token reduction when AI agents read the codebase.
*   **Intent-Driven Compilation (Telepathic AST):** You can write the `requires/ensures` contracts and leave the function body entirely blank (`...`). At compile time, the compiler spins up an embedded LLM, synthesizes the optimal algorithm, formally verifies it, and lowers it to assembly.
*   **Global Hive-Mind Fuzzing:** The compiler runs as a background daemon, connecting to a global peer-to-peer network of idle openOODA compilers. The network collaboratively generates AI-driven semantic mutations while you sleep, mathematically proving or breaking your contracts overnight.

## 3. The Safety & Security Engine

*   **Unified Capability Sandboxing:** A default-deny security model where functions cannot perform I/O without explicit capability tokens (e.g., `&NetCap`, `&FsCap`). This scales all the way up to **Biometric Attestation** (e.g., `&SysCap<RequireBiometric>`), which pauses execution to require a physical hardware enclave or FaceID scan before running critical logic.
*   **Time & Entropy Sandboxing:** Code cannot read the system clock or generate random numbers without `&TimeCap` and `&RandCap`, guaranteeing functions are mathematically pure and testing is 100% deterministic.
*   **Memory Quotas (Heap Sandboxing):** Capabilities like `&AllocCap<10MB>` mathematically restrict a specific module from allocating more than a designated RAM limit, neutralizing Out-Of-Memory (OOM) and zip-bomb attacks from 3rd-party libraries.
*   **CPU Quotas (Execution Sandboxing):** Functions can be tagged with `#[MaxCycles(5000)]`. If the static analyzer cannot mathematically prove the loop bounds will finish under that limit, it refuses to compile—making infinite loops impossible.
*   **Static Taint Tracking:** Tag sensitive variables (like passwords) with `#[Secret]`. The compiler mathematically tracks the AST flow to guarantee the data can physically never reach a function that logs to `stdout` or writes to an unencrypted `&NetCap`.
*   **Automated Contract Fuzzer:** The `ooda test --fuzz` engine continuously attempts to generate edge cases to mathematically break your `requires/ensures` contracts.
*   **0ms GC & Memory Safety:** Scope-based RAII and Automatic Reference Counting (ARC) eliminate memory leaks and completely avoid Stop-The-World garbage collection pauses.
*   **Temporal Memory (State Rollback):** "Undo for RAM." Variables store their historical states. Upon a fatal crash, the runtime can roll memory back 3 seconds to a known-safe state and execute a fallback path rather than segfaulting.
*   **Cryptographic Call-Graph Integrity:** The compiler generates a cryptographic hash of all valid function transitions. If a hacker exploits a vulnerability (e.g., a ROP attack), the CPU traps the mismatched return pointer and detonates the process.
*   **Shadow-State Semantic Reversion:** Critical modules run a few instructions ahead in a virtual "ghost state." If a mutation violates a semantic `ensures` contract, the runtime cancels the execution before the CPU commits the change to physical RAM.
*   **Polymorphic Metamorphic Binaries (Immune Systems):** Compiled binaries mutate their own assembly code every few milliseconds at runtime. The logical output remains mathematically identical, but registers and memory layouts randomize continuously, making the binary's attack surface physically impossible to target.

## 4. The Compiler & Execution Targets

*   **Multi-Target Engine Architecture:** 
    *   **Development JIT (`ooda run`):** Instant sub-millisecond execution via a built-in bytecode VM.
    *   **Production LLVM (`ooda build --emit-llvm`):** Compiles directly to native LLVM IR (`.ll`) for bare-metal CPU performance.
    *   **Universal GPU/NPU Acceleration:** Native compilation down to NVIDIA (PTX), AMD (ROCm), Intel (SPIR-V), and Apple Silicon (Metal) for zero-overhead tensor math and parallel compute—bypassing Python/C++ bindings entirely.
    *   **Direct WebAssembly (`ooda build --target wasm`):** Native WASM emission for browsers and edge environments.
    *   **Bare-Metal Embedded (`#![no_std]`):** Replaces OS capabilities with raw hardware limits (e.g., `&GpioPin4`) for IoT.
*   **Native Hot-Code Reloading:** The JIT VM supports swapping new ASTs into the running process, allowing developers to edit code without losing active application state.
*   **Advanced Toolchains:** 
    *   **Cross-Language LTO:** Link directly with C++/Rust so tightly that the LLVM backend optimizes them together with zero FFI penalty.
    *   **Deterministic Reproducible Builds:** Sandboxed compilation guarantees identical byte-for-byte hashes across all machines.
    *   **Compile-Time FFI Generation:** `import "C" "sqlite3.h"` auto-generates safe openOODA wrappers at compile time.
*   **Holographic Data Persistence:** A struct in RAM can be directly mapped to a persistent, immutable Merkle-tree on an NVMe drive. There is no `save()` function. If the server loses power, the exact memory state materializes upon boot. Data becomes immortal.

## 5. Ecosystem & Developer Experience (DX)

*   **100% Self-Hosted Ecosystem:** It is not enough for the compiler to be self-hosted. The package manager, the Language Server, the testing framework, and the network registry nodes must all be written in pure openOODA to mathematically preserve the cryptographic security chain.
*   **The Verifiable Web of Code (Zero-Trust):** A decentralized package manager where imported modules are mathematically proven by the compiler's Formal Verification solver. Third-party libraries are cryptographically minted with a strict capability manifest, allowing you to dynamically import AI-generated code with 100% mathematical trust.
*   **Fearless Concurrency:** Multithreading without Mutexes. Threads communicate via message passing, and the compiler enforces that capabilities are relinquished across thread boundaries, guaranteeing zero data races at compile time.
*   **The Standard Library Philosophy:** The standard library is strictly bifurcated into `std::core` (pure logic, requires zero capabilities, runs anywhere) and `std::os` (requires OS capabilities, runs on LLVM/JIT).
*   **Narrative Diagnostics:** When a contract or capability is violated, the compiler doesn't just throw a stack trace. It emits a causal story, tracing the exact flow of data from its origin to the violation, making complex systemic bugs trivial to read and understand.
*   **Human-in-the-Loop (`hitl`) Testing:** openOODA natively supports subjective human feedback in its testing pipeline. Primitives like `verify_human("Review this output")` allow autonomous AI fuzzer loops to pause and request your subjective approval via the CLI before marking a build as passing.
*   **Universal Native LSP:** The sub-millisecond compiler runs natively as a background daemon. Because it uses the open Language Server Protocol, it plugs instantly into Neovim, CLI-based AI agents, Cursor, or any environment you prefer. Zero editor lock-in.

---

## 6. Architectural Tension & Conflict Resolution

At the bleeding edge of systems design, extreme features often contradict each other. openOODA resolves these inherent tension points through strict boundary definitions:

*   **Metamorphic Binaries vs. Deterministic Builds:** To support supply-chain security, compilation produces a byte-for-byte deterministic hash on disk. The polymorphic metamorphism only occurs dynamically in RAM during the OS load sequence or JIT phase, ensuring reproducible builds don't fight binary immune systems.
*   **0ms GC (ARC) vs. Temporal Memory:** ARC strictly destroys standard variables the instant they leave scope. Temporal Memory rollback is an opt-in keyword (e.g., `temporal struct`). For these specific structures, ARC routes them to a ring-buffer Event Log Arena that prunes states older than 3 seconds, avoiding Use-After-Free segfaults.
*   **Capability Sandboxing vs. C/C++ FFI:** Because C/C++ code has no capability tracking and can execute arbitrary syscalls, any openOODA function that invokes Compile-Time FFI must explicitly demand an `&UnsafeFFICap` in its parameter list. The capability taint-tracking treats the FFI boundary as a deliberate, statically tracked sandbox breach.

---

## 📂 Design Document Locations & Links

* 📜 **Formal EBNF Grammar**: `ooda.ebnf`
* 📄 **Full Specification**: `SPEC.md`
* ⚙️ **Compiler Source**: `openOODA/ooda`
* 🧪 **QA Integration Suite**: `openOODA/qa`
* 🌐 **Interactive Web Playground**: `openOODA/docs/index.html`
