# examples/prototypes — illustrative only

These files are **illustrative prototypes**, not a self-hosted compiler.

The production OODA compiler lives in `../src/` (Rust). It has never been
bootstrapped through `.oo` source. The two files in this directory
demonstrate the *shape* of what a self-hosting bootstrap would look like:

- `self_hosted_lexer.oo` — keyword → token-name string mapping.
- `self_hosted_compiler.oo` — token-type → AST-node-name string mapping.

They are intentionally tiny. They exist so that you can read them and see
the kind of surface a self-hosted bootstrap would expose. They are not run
as part of the build, are not exercised by the QA suite, and are not
installed by `cargo build`.

To run the actual compiler:

```bash
cargo build --release
./target/release/ooda run examples/hello.oo
```

To run the QA suite against the real toolchain:

```bash
cd ../openooda-qa
./qa_runner.sh
```