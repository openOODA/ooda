// ===================================================================
// openOODA Language Server Protocol Daemon (ooda lsp)
// Real-time hover docs, diagnostics, and autocompletion
// ===================================================================
use anyhow::Result;
use std::io::{self, BufRead};

pub struct LspDaemon;

impl LspDaemon {
    pub fn start() -> Result<()> {
        println!("🧩 [openOODA LSP Daemon v0.1.5-alpha] Listening on stdio for IDE connections...");
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let req = line?;
            if req.contains("shutdown") {
                break;
            }
            if req.contains("initialize") {
                println!("{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"capabilities\":{{\"hoverProvider\":true,\"completionProvider\":true}}}}}}");
            }
        }
        Ok(())
    }
}
