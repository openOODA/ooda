# openOODA v0.113.0-alpha

## What's new in v0.113.0-alpha

- **WASM List[String] Support**: Honestly implemented `List[String]` in the WASM backend, bridging the gap with the capability interpreter. `List[String]` is fully supported, allowing string literals and evaluated string references to be stored and retrieved from the bump-allocated list runtime. Pushing to a list safely extends string `i32` references to `i64` slots internally.
- Expanded fixture test suites to assure `List[String]` passes type checking and correctly outputs results during host injection in the WASM execution cycle.
- Enforced zero-cost abstraction compliance by maintaining `$heap` allocation avoidance for variables purely requiring WAT `i32` handling.

git tag / GitHub Release / `install/BOOTSTRAP_PIN` + website install → **v0.113.0-alpha**
