impl LspDaemon {

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
