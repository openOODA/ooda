// ===================================================================
// openOODA LSP — honest alpha gate
// ===================================================================
use anyhow::{bail, Result};

pub struct LspDaemon;

impl LspDaemon {
    pub fn start() -> Result<()> {
        bail!(
            "ooda lsp is not implemented in this alpha (no Content-Length JSON-RPC server). \
             Use `ooda outline`, `ooda reflect`, and `ooda run --json-errors` until a real LSP ships."
        );
    }
}
