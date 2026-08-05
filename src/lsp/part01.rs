impl LspDaemon {
    pub fn start() -> Result<()> {
        eprintln!(
            "ooda lsp: textDocumentSync=Incremental with parse/cap/typecheck diagnostics \
             + WorkspaceEdit codeAction (let mut / missing return / return type / undef var) \
             (not a full language server — no completion/hover/rename)."
        );
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let mut reader = stdin.lock();
        // Open buffers: file:// URI → current full text (Full sync).
        let mut docs: HashMap<String, String> = HashMap::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            if line.starts_with("Content-Length: ") {
                let len_str = line.trim_start_matches("Content-Length: ").trim();
                let len: usize = len_str.parse().unwrap_or(0);
                let mut sep = String::new();
                reader.read_line(&mut sep)?;
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf)?;
                let json_str = String::from_utf8_lossy(&buf);
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                        if method == "initialize" || method == "shutdown" {
                            if let Some(id) = json.get("id") {
                                let res = if method == "initialize" {
                                    serde_json::json!({
                                        "capabilities": {
                                            "textDocumentSync": 2,
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

                                if let Some(uri) = uri {
                                    if method == "textDocument/didOpen" {
                                        let text = params
                                            .get("textDocument")
                                            .and_then(|t| t.get("text"))
                                            .and_then(|t| t.as_str());
                                        if let Some(text) = text {
                                            docs.insert(uri.to_string(), text.to_string());
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
                                    } else {
                                        // Incremental (or full) contentChanges
                                        if let Some(changes) =
                                            params.get("contentChanges").and_then(|c| c.as_array())
                                        {
                                            let base = docs.get(uri).map(|s| s.as_str()).unwrap_or("");
                                            let text = apply_content_changes(base, changes);
                                            docs.insert(uri.to_string(), text.clone());
                                            let diagnostics = Self::diagnose_source(&text);
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
                                }
                            }
                        } else if method == "textDocument/didClose" {
                            if let Some(uri) = json
                                .get("params")
                                .and_then(|p| p.get("textDocument"))
                                .and_then(|t| t.get("uri"))
                                .and_then(|u| u.as_str())
                            {
                                docs.remove(uri);
                            }
                        } else if method == "textDocument/codeAction" {
                            if let (Some(id), Some(params)) = (json.get("id"), json.get("params")) {
                                let uri = params
                                    .get("textDocument")
                                    .and_then(|t| t.get("uri"))
                                    .and_then(|u| u.as_str())
                                    .unwrap_or("");
                                let source = docs.get(uri).map(|s| s.as_str()).unwrap_or("");
                                let actions = Self::code_actions_for(uri, source, params);
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


    fn write_message(stdout: &mut impl Write, resp: &serde_json::Value) -> Result<()> {
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

}
