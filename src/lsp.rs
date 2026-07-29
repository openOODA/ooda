// ===================================================================
// openOODA LSP — minimal Content-Length stub (not a full language server)
// ===================================================================
use anyhow::Result;

pub struct LspDaemon;

impl LspDaemon {
    /// Stdio JSON-RPC loop that answers `initialize` / `shutdown` / `exit` only.
    /// No diagnostics, completion, hover, or didOpen analysis — use
    /// `ooda check --json-errors`, `ooda outline`, `ooda reflect` for real tooling.
    pub fn start() -> Result<()> {
        eprintln!(
            "ooda lsp: minimal stub (initialize/shutdown/exit only). \
             Full language features are not implemented — prefer ooda check/outline."
        );
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let mut reader = stdin.lock();
        loop {
            let mut line = String::new();
            if std::io::BufRead::read_line(&mut reader, &mut line)? == 0 {
                break;
            }
            if line.starts_with("Content-Length: ") {
                let len_str = line.trim_start_matches("Content-Length: ").trim();
                let len: usize = len_str.parse().unwrap_or(0);
                // Consume header separator line(s).
                let mut sep = String::new();
                std::io::BufRead::read_line(&mut reader, &mut sep)?;
                let mut buf = vec![0u8; len];
                std::io::Read::read_exact(&mut reader, &mut buf)?;
                let json_str = String::from_utf8_lossy(&buf);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                        if method == "initialize" || method == "shutdown" {
                            if let Some(id) = json.get("id") {
                                let res = if method == "initialize" {
                                    // Advertise almost nothing — honest capabilities.
                                    serde_json::json!({
                                        "capabilities": {
                                            "textDocumentSync": 0
                                        },
                                        "serverInfo": {
                                            "name": "ooda-lsp-stub",
                                            "version": env!("CARGO_PKG_VERSION")
                                        }
                                    })
                                } else {
                                    serde_json::Value::Null
                                };
                                let resp = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": res
                                });
                                let resp_str = resp.to_string();
                                use std::io::Write;
                                write!(
                                    stdout,
                                    "Content-Length: {}\r\n\r\n{}",
                                    resp_str.len(),
                                    resp_str
                                )?;
                                stdout.flush()?;
                            }
                        } else if method == "exit" {
                            break;
                        }
                        // Other methods: ignore (no response) — not implemented.
                    }
                }
            }
        }
        Ok(())
    }
}
