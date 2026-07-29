// ===================================================================
// openOODA LSP — textDocumentSync diagnostics (parse + cap + typecheck)
// ===================================================================
use anyhow::Result;
use crate::diagnostics::{parse_loc, to_lsp_position};

pub struct LspDaemon;

impl LspDaemon {
    pub fn start() -> Result<()> {
        eprintln!(
            "ooda lsp: textDocumentSync=Full with parse/cap/typecheck diagnostics \
             (not a full language server — no completion/hover/rename)."
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
                                            "textDocumentSync": 1,
                                            "codeActionProvider": true
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
                                Self::write_message(&mut stdout, &resp)?;
                            }
                        } else if method == "exit" {
                            break;
                        } else if method == "textDocument/didOpen"
                            || method == "textDocument/didChange"
                        {
                            if let Some(params) = json.get("params") {
                                let uri = params
                                    .get("textDocument")
                                    .and_then(|t| t.get("uri"))
                                    .and_then(|u| u.as_str());
                                let text = if method == "textDocument/didOpen" {
                                    params
                                        .get("textDocument")
                                        .and_then(|t| t.get("text"))
                                        .and_then(|t| t.as_str())
                                } else {
                                    // Full sync: last change carries full document text.
                                    params
                                        .get("contentChanges")
                                        .and_then(|c| c.as_array())
                                        .and_then(|c| c.last())
                                        .and_then(|c| c.get("text"))
                                        .and_then(|t| t.as_str())
                                };

                                if let (Some(uri), Some(text)) = (uri, text) {
                                    let diagnostics = Self::diagnose_source(text);
                                    let resp = serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "method": "textDocument/publishDiagnostics",
                                        "params": {
                                            "uri": uri,
                                            "diagnostics": diagnostics
                                        }
                                    });
                                    let _ = Self::write_message(&mut stdout, &resp);
                                }
                            }
                        } else if method == "textDocument/codeAction" {
                            if let (Some(id), Some(params)) = (json.get("id"), json.get("params")) {
                                let uri = params
                                    .get("textDocument")
                                    .and_then(|t| t.get("uri"))
                                    .and_then(|u| u.as_str())
                                    .unwrap_or("");
                                
                                let mut actions = vec![];
                                if let Some(diags) = params.get("context").and_then(|c| c.get("diagnostics")).and_then(|d| d.as_array()) {
                                    for d in diags {
                                        if let Some(msg) = d.get("message").and_then(|m| m.as_str()) {
                                            if msg.contains("immutable") || msg.contains("let mut") {
                                                actions.push(serde_json::json!({
                                                    "title": "Use let mut for assigned binding",
                                                    "kind": "quickfix",
                                                    "diagnostics": [d],
                                                    "command": {
                                                        "title": "Fix",
                                                        "command": "ooda.patch",
                                                        "arguments": [uri, msg]
                                                    }
                                                }));
                                            } else if msg.contains("missing return") {
                                                actions.push(serde_json::json!({
                                                    "title": "Add default return value",
                                                    "kind": "quickfix",
                                                    "diagnostics": [d],
                                                    "command": {
                                                        "title": "Fix",
                                                        "command": "ooda.patch",
                                                        "arguments": [uri, msg]
                                                    }
                                                }));
                                            } else if msg.contains("non-exhaustive match") {
                                                actions.push(serde_json::json!({
                                                    "title": "Cover all match variants",
                                                    "kind": "quickfix",
                                                    "diagnostics": [d],
                                                    "command": {
                                                        "title": "Fix",
                                                        "command": "ooda.patch",
                                                        "arguments": [uri, msg]
                                                    }
                                                }));
                                            }
                                        }
                                    }
                                }
                                let resp = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": actions
                                });
                                let _ = Self::write_message(&mut stdout, &resp);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn write_message(stdout: &mut impl std::io::Write, resp: &serde_json::Value) -> Result<()> {
        let resp_str = resp.to_string();
        write!(
            stdout,
            "Content-Length: {}\r\n\r\n{}",
            resp_str.len(),
            resp_str
        )?;
        stdout.flush()?;
        Ok(())
    }

    /// Run lex → parse → caps → typecheck; collect LSP diagnostics.
    fn diagnose_source(text: &str) -> Vec<serde_json::Value> {
        let mut diagnostics = vec![];
        let mut lexer = crate::lexer::Lexer::new(text);
        match lexer.tokenize() {
            Ok(tokens) => {
                let mut parser = crate::parser::Parser::new(tokens);
                match parser.parse_program() {
                    Ok(prog) => {
                        if let Err(e) =
                            crate::capabilities::CapabilityChecker::check_program(&prog)
                        {
                            diagnostics.push(Self::parse_diagnostic(&e.to_string()));
                        }
                        if let Err(e) = crate::typecheck::TypeChecker::check_program(&prog) {
                            diagnostics.push(Self::parse_diagnostic(&e.to_string()));
                        }
                    }
                    Err(e) => {
                        diagnostics.push(Self::parse_diagnostic(&e.to_string()));
                    }
                }
            }
            Err(e) => {
                diagnostics.push(Self::parse_diagnostic(&e.to_string()));
            }
        }
        diagnostics
    }

    /// Map compiler 1-indexed locations to LSP 0-indexed range.
    fn parse_diagnostic(msg: &str) -> serde_json::Value {
        let (line_1, col_1) = parse_loc(msg);
        let (line, character) = to_lsp_position(line_1, col_1);
        // end character: exclusive; advance one UTF-16 unit when possible (ASCII-safe).
        let end_character = character.saturating_add(1);
        serde_json::json!({
            "severity": 1,
            "message": msg,
            "range": {
                "start": { "line": line, "character": character },
                "end": { "line": line, "character": end_character }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::LspDaemon;

    #[test]
    fn parse_diagnostic_type_error_zero_index() {
        let d = LspDaemon::parse_diagnostic("Type error at 4:26: undefined variable 'foo'");
        assert_eq!(d["range"]["start"]["line"], 3);
        assert_eq!(d["range"]["start"]["character"], 25);
        assert_eq!(d["range"]["end"]["character"], 26);
    }

    #[test]
    fn parse_diagnostic_capability_zero_index() {
        let msg = "Security Capability Violation: Function 'rogue_fetch' calls sealed effectful builtin 'fetch' which requires a &NetCap parameter, but none was declared at line 2, col 52. Default-deny.";
        let d = LspDaemon::parse_diagnostic(msg);
        assert_eq!(d["range"]["start"]["line"], 1);
        assert_eq!(d["range"]["start"]["character"], 51);
    }

    #[test]
    fn parse_diagnostic_defaults_zero_zero() {
        let d = LspDaemon::parse_diagnostic("totally unstructured");
        // source (1,1) → LSP (0,0)
        assert_eq!(d["range"]["start"]["line"], 0);
        assert_eq!(d["range"]["start"]["character"], 0);
    }

    #[test]
    fn diagnose_source_flags_type_error() {
        let diags = LspDaemon::diagnose_source("pub fn main() { println(1 + \"a\"); }\n");
        assert!(!diags.is_empty(), "expected type diagnostic");
    }

    #[test]
    fn diagnose_source_flags_cap_error() {
        let diags = LspDaemon::diagnose_source(
            "pub fn main() { let _ = fetch(\"https://x\"); }\n",
        );
        assert!(
            diags.iter().any(|d| d["message"]
                .as_str()
                .unwrap_or("")
                .contains("Capability")
                || d["message"].as_str().unwrap_or("").contains("fetch")),
            "expected cap diagnostic: {:?}",
            diags
        );
    }
}
