// ===================================================================
// openOODA LSP — AI Diagnostics via `textDocument/didOpen|didChange`
// ===================================================================
use anyhow::Result;

pub struct LspDaemon;

impl LspDaemon {
    pub fn start() -> Result<()> {
        eprintln!(
            "ooda lsp: starting with full textDocumentSync for live diagnostics."
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
                                    serde_json::json!({
                                        "capabilities": {
                                            "textDocumentSync": 1 // Full
                                        },
                                        "serverInfo": {
                                            "name": "ooda-lsp",
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
                        } else if method == "textDocument/didOpen" || method == "textDocument/didChange" {
                            if let Some(params) = json.get("params") {
                                let uri = if method == "textDocument/didOpen" {
                                    params.get("textDocument").and_then(|t| t.get("uri")).and_then(|u| u.as_str())
                                } else {
                                    params.get("textDocument").and_then(|t| t.get("uri")).and_then(|u| u.as_str())
                                };
                                let text = if method == "textDocument/didOpen" {
                                    params.get("textDocument").and_then(|t| t.get("text")).and_then(|t| t.as_str())
                                } else {
                                    params.get("contentChanges")
                                        .and_then(|c| c.as_array())
                                        .and_then(|c| c.first())
                                        .and_then(|c| c.get("text"))
                                        .and_then(|t| t.as_str())
                                };

                                if let (Some(uri), Some(text)) = (uri, text) {
                                    let mut diagnostics = vec![];
                                    let mut lexer = crate::lexer::Lexer::new(text);
                                    if let Ok(tokens) = lexer.tokenize() {
                                        let mut parser = crate::parser::Parser::new(tokens);
                                        match parser.parse_program() {
                                            Ok(prog) => {
                                                if let Err(e) = crate::typecheck::TypeChecker::check_program(&prog) {
                                                    diagnostics.push(Self::parse_diagnostic(&e.to_string()));
                                                }
                                            }
                                            Err(e) => {
                                                diagnostics.push(Self::parse_diagnostic(&e.to_string()));
                                            }
                                        }
                                    } else {
                                        diagnostics.push(Self::parse_diagnostic("parse error at 1:1: Invalid token"));
                                    }

                                    let resp = serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "method": "textDocument/publishDiagnostics",
                                        "params": {
                                            "uri": uri,
                                            "diagnostics": diagnostics
                                        }
                                    });
                                    let resp_str = resp.to_string();
                                    use std::io::Write;
                                    let _ = write!(
                                        stdout,
                                        "Content-Length: {}\r\n\r\n{}",
                                        resp_str.len(),
                                        resp_str
                                    );
                                    let _ = stdout.flush();
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_diagnostic(msg: &str) -> serde_json::Value {
        let mut l = 1;
        let mut c = 1;
        if let Some(idx) = msg.find(" at ") {
            let rest = &msg[idx + 4..];
            let coords: String = rest.chars().take_while(|ch| ch.is_ascii_digit() || *ch == ':').collect();
            let parts: Vec<&str> = coords.split(':').collect();
            if parts.len() >= 2 {
                if let (Ok(parsed_l), Ok(parsed_c)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    l = parsed_l;
                    c = parsed_c;
                }
            }
        }
        let line = l.saturating_sub(1);
        let char = c.saturating_sub(1);
        serde_json::json!({
            "severity": 1,
            "message": msg,
            "range": {
                "start": { "line": line, "character": char },
                "end": { "line": line, "character": char + 1 }
            }
        })
    }
}
