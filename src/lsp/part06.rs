
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

    #[test]
    fn apply_content_changes_incremental_replace() {
        let base = "pub fn main() {\n    println(1);\n}\n";
        let changes = vec![serde_json::json!({
            "range": {
                "start": { "line": 1, "character": 12 },
                "end": { "line": 1, "character": 13 }
            },
            "text": "42"
        })];
        let out = super::apply_content_changes(base, &changes);
        assert!(out.contains("println(42)"), "got:\n{}", out);
    }

    #[test]
    fn apply_content_changes_clamps_character_past_eol() {
        let base = "ab\ncd\n";
        // Replace "spilled" range that claims character 999 on line 0 → clamp to end of "ab"
        let changes = vec![serde_json::json!({
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 999 }
            },
            "text": "XY"
        })];
        let out = super::apply_content_changes(base, &changes);
        assert_eq!(out, "XY\ncd\n", "got {:?}", out);
    }

    #[test]
    fn code_action_undefined_var_preserves_indent() {
        let src = "pub fn main() {\n    println(foo);\n}\n";
        let params = serde_json::json!({
            "context": {
                "diagnostics": [{
                    "message": "Type error at 2:12: undefined variable 'foo'"
                }]
            }
        });
        let actions = LspDaemon::code_actions_for("file:///u.oo", src, &params);
        assert!(!actions.is_empty(), "{:?}", actions);
        let text = actions[0]["edit"]["changes"]["file:///u.oo"][0]["newText"]
            .as_str()
            .unwrap();
        assert!(
            text.starts_with("    let mut foo = 0;"),
            "expected indented stub, got {:?}",
            text
        );
    }

    #[test]
    fn code_action_arg_type_replaces_whole_string_token() {
        // Diagnostic lands on call-site `)` (col 26 1-index for f("hello"))
        let src = "pub fn f(x: Int) { println(x); }\npub fn main() { f(\"hello\"); }\n";
        let params = serde_json::json!({
            "context": {
                "diagnostics": [{
                    "message": "Type error at 2:26: function 'f' argument 0 expects Int, found String"
                }]
            }
        });
        let actions = LspDaemon::code_actions_for("file:///a.oo", src, &params);
        assert_eq!(actions.len(), 1, "{:?}", actions);
        let edit = &actions[0]["edit"]["changes"]["file:///a.oo"][0];
        assert_eq!(edit["newText"], "0");
        let start_c = edit["range"]["start"]["character"].as_u64().unwrap();
        let end_c = edit["range"]["end"]["character"].as_u64().unwrap();
        assert_eq!(end_c - start_c, 7, "start={} end={} edit={:?}", start_c, end_c, edit);
        let line = "pub fn main() { f(\"hello\"); }";
        let mut applied = line.to_string();
        applied.replace_range(start_c as usize..end_c as usize, "0");
        assert!(applied.contains("f(0)"), "applied={}", applied);
    }

    #[test]
    fn scan_token_end_string_and_ident() {
        assert_eq!(super::scan_token_end(r#""ab" rest"#, 0), Some(4));
        assert_eq!(super::scan_token_end("foo bar", 0), Some(3));
        assert_eq!(super::scan_token_end("42,", 0), Some(2));
        let line = r#"pub fn main() { f("hello"); }"#;
        let at = line.rfind(')').unwrap(); // call-site `)`, not `main()`
        let (s, e) = super::arg_token_span_near(line, at).unwrap();
        assert_eq!(&line[s..e], "\"hello\"");
    }
}

