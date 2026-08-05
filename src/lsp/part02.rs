impl LspDaemon {

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
            if msg.contains("undefined variable") {
                if let Some(edit) = Self::undefined_var_edit(uri, source, msg) {
                    actions.push(serde_json::json!({
                        "title": "Declare undefined variable with default value",
                        "kind": "quickfix",
                        "diagnostics": [d],
                        "edit": edit
                    }));
                }
            }
            if msg.contains("Expected token Colon") && msg.contains("found RParen") {
                if let Some(edit) = Self::arg_type_missing_edit(uri, source, msg) {
                    actions.push(serde_json::json!({
                        "title": "Add default Int type annotation",
                        "kind": "quickfix",
                        "diagnostics": [d],
                        "edit": edit
                    }));
                }
            }
            if msg.contains(" expects ") && msg.contains(", found ") && msg.contains("argument") {
                if let Some(edit) = Self::arg_type_mismatch_edit(uri, source, msg) {
                    actions.push(serde_json::json!({
                        "title": "Replace argument with default of expected type",
                        "kind": "quickfix",
                        "diagnostics": [d],
                        "edit": edit
                    }));
                }
            }
        }
        actions
    }


    /// Replace the whole argument token with a typed default.
    /// Message: `function 'f' argument 0 expects Int, found String`.
    /// Typechecker locations typically land on the call's closing `)`; we walk back
    /// to the preceding simple token (string / number / ident). Not first-char theater.
    fn arg_type_mismatch_edit(
        uri: &str,
        source: &str,
        msg: &str,
    ) -> Option<serde_json::Value> {
        if source.is_empty() {
            return None;
        }
        let expected = msg
            .split(" expects ")
            .nth(1)?
            .split(',')
            .next()?
            .trim();
        let default_val = match expected {
            "Int" => "0",
            "String" => "\"\"",
            "Bool" => "false",
            "Float" => "0.0",
            _ => return None,
        };
        let (line_1, col_1) = parse_loc(msg);
        let line_0 = line_1.saturating_sub(1);
        let col_0 = col_1.saturating_sub(1);
        let at = lsp_position_to_byte_offset(source, line_0, col_0);
        let (start, end) = arg_token_span_near(source, at)?;
        if end <= start {
            return None;
        }
        let (sl, sc) = byte_offset_to_lsp(source, start);
        let (el, ec) = byte_offset_to_lsp(source, end);
        let text_edit = serde_json::json!({
            "range": {
                "start": { "line": sl, "character": sc },
                "end": { "line": el, "character": ec }
            },
            "newText": default_val
        });
        let mut changes = serde_json::Map::new();
        changes.insert(uri.to_string(), serde_json::Value::Array(vec![text_edit]));
        Some(serde_json::json!({ "changes": changes }))
    }


    /// Insert `: Int` at the syntax error location for missing parameter types.
    fn arg_type_missing_edit(
        uri: &str,
        source: &str,
        msg: &str,
    ) -> Option<serde_json::Value> {
        if source.is_empty() {
            return None;
        }
        let (line_1, col_1) = parse_loc(msg);
        let line_0 = line_1.saturating_sub(1);
        let col_0 = col_1.saturating_sub(1);
        
        let new_text = ": Int".to_string();
        let text_edit = serde_json::json!({
            "range": {
                "start": { "line": line_0, "character": col_0 },
                "end": { "line": line_0, "character": col_0 }
            },
            "newText": new_text
        });
        let mut changes = serde_json::Map::new();
        changes.insert(uri.to_string(), serde_json::Value::Array(vec![text_edit]));
        Some(serde_json::json!({ "changes": changes }))
    }


    /// Insert `let mut x = 0;\n` at the start of the diagnostic line, preserving indent.
    fn undefined_var_edit(
        uri: &str,
        source: &str,
        msg: &str,
    ) -> Option<serde_json::Value> {
        if source.is_empty() {
            return None;
        }
        let var_name = msg
            .split("undefined variable '")
            .nth(1)
            .and_then(|s| s.split('\'').next())
            .unwrap_or("");
        if var_name.is_empty() {
            return None;
        }
        let (line_1, _) = parse_loc(msg);
        let line_0 = line_1.saturating_sub(1);
        let indent = line_indent(source, line_0);
        let new_text = format!("{}let mut {} = 0;\n", indent, var_name);
        let text_edit = serde_json::json!({
            "range": {
                "start": { "line": line_0, "character": 0 },
                "end": { "line": line_0, "character": 0 }
            },
            "newText": new_text
        });
        let mut changes = serde_json::Map::new();
        changes.insert(uri.to_string(), serde_json::Value::Array(vec![text_edit]));
        Some(serde_json::json!({ "changes": changes }))
    }

}
