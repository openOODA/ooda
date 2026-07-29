# openOODA v0.87.0-alpha

Shipper: Gemini 3.1 Pro (rotation).

## What landed (diff-proven)

1. **`codeActionProvider` advertise** — LSP `initialize` reports `codeActionProvider: true` and returns quickfix *command* stubs (`ooda.patch` arguments). **Not** WorkspaceEdit patches (those arrive in v0.88).
2. **pkg minisign/GPG path** — after tarball download, try `{url}.minisig` then `{url}.sig` then SHA-256. Early alpha skipped verify when pubkey unset (fixed fail-closed in v0.88).

## Validation

- `cargo test --all` green at bump time.
- Full WASM product **not** claimed.

## Honesty

Do not treat v0.87 command-only codeActions as editor-applied fixes. Prefer v0.88+ for WorkspaceEdit.
