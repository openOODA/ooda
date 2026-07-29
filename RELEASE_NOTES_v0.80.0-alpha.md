# openOODA v0.80.0-alpha

## Features
- Implemented `for item in list` parsing and desugaring in the interpreter and C codegen.
- Lists can now be safely iterated over without resorting to manual index tracking.

## Fixes
- Re-enforced strict bounds checking on `list_get` and `list_len` desugaring.
