# User structs — path A (M168)

**Status:** source floor **In** for Backend-C typedef emit + field assign.  
**Marker:** `STRUCT_PATH_A_ALPHA`

## Product floor (after pure rebuild of tip oodac)

- `type Name = struct { f: T, ... };` → C `typedef struct { … } Name;`
- Simple aliases `type Byte = Int;` → `typedef long long Byte;`
- `c_ty_at` returns user type names (not always `long long`)
- Struct lit bind `let p = Point { … }` uses C type `Point`
- Field assign `m.v = expr;` emitted (was silent `m.v;`)
- Param/field name shadow: `get_b(b: Box) { return (b).b; }` typechecks
- `List[Int]` / `List[String]` as struct fields → `OoIList` / `OoSList`

## Residual (do not claim)

- **Tip host lag:** until pure multi rebuild lands, product `oodac` may still emit `((Box){…})` without `typedef` (check still OK)
- **`List[Struct]`** nested product (e.g. `List[Node]`) — not Backend-C list ABI
- **`&mut T`** mutable references — by-value mut + return is path A
- Full generics / trait objects

## Rails

- `scripts/agy_lang_blockers_smoke.sh`
- Fixtures: `fixtures/agy_int_lt0.oo`, `fixtures/agy_struct_path_a.oo`
- Source: `oodac/c_emit_struct.oo`, `c_emit.oo`, `c_emit_ident.oo`, `c_emit_skip.oo` (`c_ty_at`)
