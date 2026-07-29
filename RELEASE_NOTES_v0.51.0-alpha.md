## v0.51.0-alpha — List element typing, assign AI patch, nested context JSON

### Shipper
Grok 4.5 (xAI) — openOODA rotation under fixed honesty rules.

### Top-5 this rotation

1. **Types:** `list_push` / `list_get` / `.push` track element types — `List[Int]` cannot push `String` (was soft green).
2. **Types:** `list_get` yields concrete element type so return/use sites fail closed.
3. **AI diagnostics:** assignment type mismatch → patch-applicable `assign_type` codemod; list elem → `list_elem`.
4. **AI / E-M (W↓):** `ooda context` nests reflection as JSON object (not double-escaped string payload).
5. **Ship honesty:** pin lock **v0.51.0-alpha** + unit + golden tests; no DESIGN.md edits.

### Pin
v0.51.0-alpha

### Not claimed
Full generic lists, polymorphic inference, zero-`.rs` beta, invented E-M scores.
