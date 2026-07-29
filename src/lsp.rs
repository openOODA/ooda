// ===================================================================
// openOODA LSP — textDocumentSync diagnostics + WorkspaceEdit codeActions
// ===================================================================
//
// Honest surface (not a full language server):
//   - textDocumentSync Full: didOpen / didChange → parse + cap + typecheck
//   - textDocument/codeAction: real WorkspaceEdit for let→let mut (via migrate)
//     and missing-return default inserts. No completion / hover / rename.
//
// Document texts live in a process-local HashMap (uri → source). codeAction
// consults that store — never re-reads disk (editor buffer is source of truth).
// ===================================================================
use anyhow::Result;
use crate::diagnostics::{byte_offset_to_lsp, parse_loc, to_lsp_position};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};

pub struct LspDaemon;

impl LspDaemon {
    pub fn start() -> Result<()> {
        eprintln!(
            "ooda lsp: textDocumentSync=Full with parse/cap/typecheck diagnostics \
             + WorkspaceEdit codeAction for let mut / missing return \
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

    /// Build WorkspaceEdit code actions from open buffer + client diagnostics.
    fn code_actions_for(
        uri: &str,
        source: &str,
        params: &serde_json::Value,
    ) -> Vec<serde_json::Value> {
        let mut actions = Vec::new();
        let diags = params
            .get("context")
            .and_then(|c| c.get("diagnostics"))
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default();

        let needs_let_mut = diags.iter().any(|d| {
            d.get("message")
                .and_then(|m| m.as_str())
                .map(|m| m.contains("immutable") || m.contains("let mut"))
                .unwrap_or(false)
        });

        if needs_let_mut && !source.is_empty() {
            if let Ok(edits) = crate::migrate::suggest_let_mut_edits(source) {
                if !edits.is_empty() {
                    let text_edits: Vec<serde_json::Value> = edits
                        .iter()
                        .map(|(start, end, text)| {
                            let (sl, sc) = byte_offset_to_lsp(source, *start);
                            let (el, ec) = byte_offset_to_lsp(source, *end);
                            serde_json::json!({
                                "range": {
                                    "start": { "line": sl, "character": sc },
                                    "end": { "line": el, "character": ec }
                                },
                                "newText": text
                            })
                        })
                        .collect();
                    let related: Vec<serde_json::Value> = diags
                        .iter()
                        .filter(|d| {
                            d.get("message")
                                .and_then(|m| m.as_str())
                                .map(|m| m.contains("immutable") || m.contains("let mut"))
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect();
                    let mut changes = serde_json::Map::new();
                    changes.insert(uri.to_string(), serde_json::Value::Array(text_edits));
                    actions.push(serde_json::json!({
                        "title": "Use let mut for assigned binding",
                        "kind": "quickfix",
                        "diagnostics": related,
                        "edit": { "changes": changes }
                    }));
                }
            }
        }

        for d in &diags {
            let msg = match d.get("message").and_then(|m| m.as_str()) {
                Some(m) => m,
                None => continue,
            };
            if msg.contains("missing return") {
                if let Some(edit) = Self::missing_return_edit(uri, source, msg) {
                    actions.push(serde_json::json!({
                        "title": "Insert default return value",
                        "kind": "quickfix",
                        "diagnostics": [d],
                        "edit": edit
                    }));
                }
            }
            if msg.contains("does not match declared") && msg.contains("return type") {
                if let Some(edit) = Self::return_type_edit(uri, source, msg) {
                    actions.push(serde_json::json!({
                        "title": "Change declared return type to match body",
                        "kind": "quickfix",
                        "diagnostics": [d],
                        "edit": edit
                    }));
                }
            }
        }
        actions
    }

    /// Change `-> Declared` to `-> Found` when body return type mismatches.
    /// Message: `return type String does not match declared Int` (+ optional `in 'name'`).
    fn return_type_edit(
        uri: &str,
        source: &str,
        msg: &str,
    ) -> Option<serde_json::Value> {
        if source.is_empty() {
            return None;
        }
        // "return type FOUND does not match declared DECLARED"
        let found = msg
            .split("return type ")
            .nth(1)?
            .split(" does not match")
            .next()?
            .trim();
        let declared = msg.split("declared ").nth(1)?.split_whitespace().next()?;
        if found.is_empty() || declared.is_empty() || found == declared {
            return None;
        }
        let fname = msg
            .split(" in '")
            .nth(1)
            .and_then(|s| s.split('\'').next())
            .unwrap_or("");
        let (start, end) = find_fn_return_type_span(source, fname, declared)?;
        let (sl, sc) = byte_offset_to_lsp(source, start);
        let (el, ec) = byte_offset_to_lsp(source, end);
        let text_edit = serde_json::json!({
            "range": {
                "start": { "line": sl, "character": sc },
                "end": { "line": el, "character": ec }
            },
            "newText": found
        });
        let mut changes = serde_json::Map::new();
        changes.insert(uri.to_string(), serde_json::Value::Array(vec![text_edit]));
        Some(serde_json::json!({ "changes": changes }))
    }

    /// Insert a typed default `return …;` just before the function's closing `}`.
    /// Message shape: `function declares return type Int but body has type Void (missing return value)`
    /// with function name in `Type error in 'NAME': …`.
    fn missing_return_edit(
        uri: &str,
        source: &str,
        msg: &str,
    ) -> Option<serde_json::Value> {
        if source.is_empty() {
            return None;
        }
        let fname = msg
            .split("Type error in '")
            .nth(1)
            .and_then(|s| s.split('\'').next())
            .unwrap_or("");
        if fname.is_empty() {
            return None;
        }
        let ret_default = if msg.contains("return type Int") {
            "0"
        } else if msg.contains("return type Bool") {
            "false"
        } else if msg.contains("return type Float") {
            "0.0"
        } else if msg.contains("return type String") {
            "\"\""
        } else {
            return None;
        };
        let body_close = find_fn_body_close(source, fname)?;
        let (line, character) = byte_offset_to_lsp(source, body_close);
        let indent = indent_before(source, body_close);
        let new_text = format!("{}return {};\n{}", indent, ret_default, indent);
        let text_edit = serde_json::json!({
            "range": {
                "start": { "line": line, "character": character },
                "end": { "line": line, "character": character }
            },
            "newText": new_text
        });
        let mut changes = serde_json::Map::new();
        changes.insert(uri.to_string(), serde_json::Value::Array(vec![text_edit]));
        Some(serde_json::json!({ "changes": changes }))
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

/// Locate the declared return type text after `->` for `fn name` (or first `fn` if name empty).
/// Returns half-open byte range of the type token(s) (simple: single identifier / bare type).
fn find_fn_return_type_span(
    source: &str,
    name: &str,
    expected_declared: &str,
) -> Option<(usize, usize)> {
    let needle = if name.is_empty() {
        "fn ".to_string()
    } else {
        format!("fn {}(", name)
    };
    let start = source.find(&needle)?;
    let after = &source[start..];
    let arrow = after.find("->")?;
    let mut i = start + arrow + 2;
    while i < source.len() && source.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    let type_start = i;
    // Consume a simple type name / bracketed form until space or `{` or requires/ensures.
    let rest = &source[type_start..];
    let mut end = 0usize;
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    while end < bytes.len() {
        let b = bytes[end];
        if b == b'[' {
            depth += 1;
            end += 1;
            continue;
        }
        if b == b']' {
            depth -= 1;
            end += 1;
            continue;
        }
        if depth == 0
            && (b.is_ascii_whitespace()
                || b == b'{'
                || rest[end..].starts_with("requires")
                || rest[end..].starts_with("ensures"))
        {
            break;
        }
        end += 1;
    }
    if end == 0 {
        return None;
    }
    let span = &source[type_start..type_start + end];
    if !span.starts_with(expected_declared) {
        // Still accept if declared type is a prefix (e.g. Int vs Int[0..10])
        if span != expected_declared {
            return None;
        }
    }
    Some((type_start, type_start + expected_declared.len().min(end)))
}

/// Find the byte index of the closing `}` of `fn NAME` / `pub fn NAME` body.
fn find_fn_body_close(source: &str, name: &str) -> Option<usize> {
    let patterns = [format!("fn {}(", name), format!("fn {} (", name)];
    let mut start = None;
    for p in &patterns {
        if let Some(idx) = source.find(p.as_str()) {
            start = Some(idx);
            break;
        }
    }
    let start = start?;
    let brace = source[start..].find('{')? + start;
    let mut depth: i32 = 0;
    let bytes = source.as_bytes();
    let mut i = brace;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'"' => {
                // skip string
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        break;
                    }
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn indent_before(source: &str, byte: usize) -> String {
    let line_start = source[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = &source[line_start..byte];
    let n = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    // Body indent is typically closing-brace indent + 4 spaces.
    format!("{}    ", &line[..n])
}

#[cfg(test)]
mod tests {
    use super::{find_fn_body_close, LspDaemon};
    use crate::diagnostics::byte_offset_to_lsp;

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
    fn code_action_let_mut_emits_workspace_edit() {
        let src = "pub fn main() {\n    let x = 1;\n    x = 2;\n    println(x);\n}\n";
        let params = serde_json::json!({
            "context": {
                "diagnostics": [{
                    "message": "Type error at 2:14: cannot assign to immutable binding 'x'; use `let mut x`"
                }]
            }
        });
        let actions = LspDaemon::code_actions_for("file:///t.oo", src, &params);
        assert_eq!(actions.len(), 1, "expected one let-mut action: {:?}", actions);
        let edit = &actions[0]["edit"]["changes"]["file:///t.oo"];
        assert!(edit.is_array());
        let arr = edit.as_array().unwrap();
        assert!(!arr.is_empty());
        assert_eq!(arr[0]["newText"], "mut ");
        // Insert after "let " on the line with `let x`
        let start_line = arr[0]["range"]["start"]["line"].as_u64().unwrap();
        assert_eq!(start_line, 1); // second line (0-indexed)
    }

    #[test]
    fn code_action_missing_return_inserts_return_zero() {
        let src = "pub fn f() -> Int {\n    let x = 1;\n}\npub fn main() { println(f()); }\n";
        let params = serde_json::json!({
            "context": {
                "diagnostics": [{
                    "message": "Type error in 'f': function declares return type Int but body has type Void (missing return value)"
                }]
            }
        });
        let actions = LspDaemon::code_actions_for("file:///m.oo", src, &params);
        assert_eq!(actions.len(), 1, "expected missing-return action: {:?}", actions);
        let edits = actions[0]["edit"]["changes"]["file:///m.oo"]
            .as_array()
            .expect("edits array");
        let text = edits[0]["newText"].as_str().unwrap();
        assert!(
            text.contains("return 0;"),
            "expected return 0 insert, got {:?}",
            text
        );
    }

    #[test]
    fn find_fn_body_close_basic() {
        let src = "pub fn f() -> Int {\n    let x = 1;\n}\n";
        let close = find_fn_body_close(src, "f").expect("close");
        assert_eq!(&src[close..close + 1], "}");
        let (line, _) = byte_offset_to_lsp(src, close);
        assert_eq!(line, 2);
    }

    #[test]
    fn code_action_no_source_yields_empty() {
        let params = serde_json::json!({
            "context": {
                "diagnostics": [{
                    "message": "cannot assign to immutable binding 'x'; use `let mut x`"
                }]
            }
        });
        let actions = LspDaemon::code_actions_for("file:///t.oo", "", &params);
        assert!(actions.is_empty());
    }

    #[test]
    fn code_action_return_type_mismatch_workspace_edit() {
        let src = "pub fn f() -> Int {\n    return \"x\";\n}\npub fn main() { println(1); }\n";
        let params = serde_json::json!({
            "context": {
                "diagnostics": [{
                    "message": "Type error at 2:15 in 'f': return type String does not match declared Int"
                }]
            }
        });
        let actions = LspDaemon::code_actions_for("file:///r.oo", src, &params);
        assert_eq!(actions.len(), 1, "expected return-type action: {:?}", actions);
        let edits = actions[0]["edit"]["changes"]["file:///r.oo"]
            .as_array()
            .expect("edits");
        assert_eq!(edits[0]["newText"], "String");
        // Apply mentally: `-> Int` becomes `-> String`
        let start_ch = edits[0]["range"]["start"]["character"].as_u64().unwrap();
        assert!(start_ch > 0);
    }
}
