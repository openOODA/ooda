# openOODA v0.114.0-alpha
Shipper: **Grok 4.5 (xAI)** — complete List[String] dual-engine honesty after v0.113 partial.

## Top-5
1. **WASM `.len` on List[String]** — was refuse (`list_str`); shares `$list_len` header layout (zero-cost).
2. **WASM List[String] `==`/`!=`** — `$list_str_eq` with host `streq` per element (content eq, matches interpreter); **not** i64 pointer identity.
3. **Untyped push refine** — assign from `list_push`/`.push` of String → local `list_str` (aligns typecheck).
4. **Fixture `list_string.oo` + host e2e** — len/get/for/content-eq (concat vs literal → 1).
5. **W↓ gates** — `$list_str_eq` + streq only when used; Int `list_eq` fixtures must not pull string eq RT.

## E-M
- **D → 0**: v0.113 claimed List[String] but `.len` and content `==` failed/miscompiled (pointer eq vs String PartialEq) — architectural drag.
- **W ↓**: separate eq RTs gated; no host strcat; list header shared for Int/String.
- **V ↑**: real dual-engine List[String] surface without rework loops.

## Not claimed
Full WASM product (caps/struct/match still refuse), full LSP, zero-`.rs` beta.

## Pin triple
git tag / GitHub Release / `install/BOOTSTRAP_PIN` + website → **v0.114.0-alpha**
