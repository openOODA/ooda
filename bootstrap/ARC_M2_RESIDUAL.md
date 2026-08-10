# M2 ARC (honest status)

**Status:** free reclaim **shipped** for product self-host (stage-2 fixed-point under free). Nested-scope + shadow + while-mut rebind fixed. Not beta.

## Shipped

| Piece | Behavior |
|-------|----------|
| `PURE_NO_ARC=0` default | Retain/release kept in pure C |
| Runtime release | **free on ref 0** for str + ilist + slist (slist releases elements first) |
| Poison | `flags = 0xFFFFFFFFu` before free; hdr_ok rejects poisoned/static |
| Nested bare blocks | `c_emit_stmt` LBRACE → `c_emit_block` only (no all-scope UAF) |
| Top-frame `c_env_put` | `let` shadow binds only in top `{;` frame |
| Reassign `c_env_put_last` | Updates last binding in any frame — while/if mut rebind no longer top-appends |
| Headered crypto strings | `oo_str_alloc_payload` for sha256/hmac/json_format_string |
| `arc_smoke.sh` | 6 fixtures incl. nested_scope + nested_shadow |
| Self-host free | stage-2 digests match; list-push stress 1000 under free |

## Root causes closed this cycle

1. **While/if mut UAF:** reassignment used top-frame `c_env_put`, so `scope_exit` freed outer `mut` each iteration.
2. **Nested bare block:** `c_emit_all_scope_releases` freed outer locals at `}`.
3. **Headerless malloc strings:** crypto/json free of `data-8` corrupted heap.

## Residual (not M2 gate)

1. Seed cold bootstrap may still mis-type newer modules — prefer tree host for pure multi.
2. Softeners (`pure_rewrite_*.py`) still residual regex, not full ownership analysis.
3. Temp `oo_str_lit` in `slist_push` args may over-retain (leak of +1), not free-unsound.
4. Not beta.

## Rebuild

```bash
export PURE_NO_ARC=0 PURE_SKIP_CHECK=1
# Prefer tree host (seed may lag):
OODAC_BIN=./oodac/oodac bash scripts/oodac_pure_build.sh oodac/main.oo oodac/oodac
# fixed-point
OODAC_BIN=./oodac/oodac bash scripts/oodac_pure_build.sh oodac/main.oo /tmp/oodac_s2
bash scripts/arc_smoke.sh
```

## Related

- `runtime/chs_rt_str.c`, `chs_rt_list.c`, `chs_rt_crypto.c`
- `oodac/c_emit_env.oo` (`c_env_put` / `c_env_put_last`)
- `oodac/c_emit_stmt.oo`, `oodac/c_emit_ident.oo`
