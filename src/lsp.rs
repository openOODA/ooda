// ===================================================================
// openOODA LSP — honest alpha gate
// ===================================================================
use anyhow::{bail, Result};

pub struct LspDaemon;

impl LspDaemon {
    pub fn start() -> Result<()> {
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
                std::io::BufRead::read_line(&mut reader, &mut line)?; // empty line
                let mut buf = vec![0; len];
                std::io::Read::read_exact(&mut reader, &mut buf)?;
                let json_str = String::from_utf8_lossy(&buf);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                        if method == "initialize" || method == "shutdown" {
                            if let Some(id) = json.get("id") {
                                let res = if method == "initialize" {
                                    serde_json::json!({ "capabilities": { "textDocumentSync": 1 } })
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
                                write!(stdout, "Content-Length: {}\r\n\r\n{}", resp_str.len(), resp_str)?;
                                stdout.flush()?;
                            }
                        } else if method == "exit" {
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
