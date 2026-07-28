# OODA Programming Language (`.oo`)
**openOODA Project** — `https://github.com/openOODA`

OODA (Observe, Orient, Decide, Act) is a modern, systems-oriented, guard-rail-first programming language designed for high reliability, capability security, zero-day defense, self-verification, and rapid AI co-authoring ("vibe coding").

---

## ⚡ Quick Start

```bash
# Clone the private repository
git clone https://github.com/openOODA/ooda.git
cd ooda

# Build the compiler toolchain
cargo build --release

# Run a .oo file using the instant JIT interpreter
./target/release/ooda run examples/hello.oo

# Run contracts and inline verify tests
./target/release/ooda test examples/math_contract.oo

# Extract token-minimized module outline for AI context
./target/release/ooda outline examples/security_cap.oo
```

---

## 📂 Project Structure

* **`ooda.ebnf`** — Ultra-compact formal EBNF grammar (<2,000 tokens).
* **`examples/`** — Reference `.oo` programs demonstrating contracts, capability security, and inline tests.
* **`src/`** — Core Rust compiler, JIT engine, and CLI toolchain.
