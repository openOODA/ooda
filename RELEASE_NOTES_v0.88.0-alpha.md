# openOODA v0.88.0-alpha

Shipper: **Grok 4.5 (xAI)** — openOODA rotation.

## Top-5 (diff-proven)

1. **Real LSP WorkspaceEdit `codeAction` for `let mut`:** open-document store on didOpen/didChange; `migrate::suggest_let_mut_edits` produces insert-`mut ` byte ranges; converted via `byte_offset_to_lsp` (UTF-16). No command-stub theater.
2. **Missing-return WorkspaceEdit:** insert typed default (`return 0;` / `false` / `0.0` / `""`) before the named function's closing `}`.
3. **pkg signature fail-closed:** present `.minisig` without `OODA_PKG_MINISIGN_PUBKEY` fails install; missing `minisign`/`gpg` with sidecar present fails; `OODA_PKG_ALLOW_UNSIGNED=1` is the only opt-out. SHA-256 path unchanged.
4. **WASM range-for coverage:** `for i in lo..hi` desugars to while; golden asserts WAT loop + break machinery (still not a full WASM product).
5. **Shared byte→LSP helper + honesty:** `byte_offset_to_lsp` unit-tested; README/notes distinguish WorkspaceEdit from command stubs.

## E-M (Ps = V · (T − D) / W)

- **D↓:** editor applies real text edits (no dead `ooda.patch` command); signature sidecars cannot be silently ignored; WASM for-range fails closed or lowers correctly.
- **W↓:** document store holds one String per open URI; edits are stack-local rewrite vectors; no extra AST materialization beyond one parse for suggestions.
- **V↑ / T:** same dual-engine surface; quicker AI fix loop via WorkspaceEdit; range-for on WASM path without new heap IR.

## Pin

`v0.88.0-alpha` — tag, GitHub Release, `install/BOOTSTRAP_PIN`, site `install.sh` must match.

## Not claimed

Full LSP (completion/hover/rename), full package registry, full WASM product (strings/lists/caps still refuse), time-travel replay, CPython embed, full SPEC self-host.
