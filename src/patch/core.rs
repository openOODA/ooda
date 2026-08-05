// ===================================================================
// openOODA surgical source patcher
//
// JSON patches target a named function and may change:
//   - body (new_body / legacy new_body_expr)
//   - parameter list (new_params)
//   - return type (new_return_type)
//   - requires / ensures contract lines (new_requires / new_ensures)
//
// After text edits: lex → parse → capability check → typecheck, then write.
// Fail closed if validation fails (no half-applied write).
// ===================================================================
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::lexer::Lexer;
use crate::parser::Parser;

#[derive(Debug, Serialize, Deserialize)]
pub struct AstPatch {
    pub target_function: String,
    /// Replacement body source (statements inside the function braces).
    pub new_body: Option<String>,
    /// Legacy: single expression treated as `return <expr>;`.
    pub new_body_expr: Option<String>,
    /// Replacement parameter list *without* outer parens, e.g. `"a: Float, b: Float"`.
    pub new_params: Option<String>,
    /// Replacement return type text, e.g. `"Float"` or `"Result[Int, String]"`.
    pub new_return_type: Option<String>,
    /// Full requires clause lines (without trailing `{`), e.g. `"requires a >= 0\nrequires b != 0"`.
    /// Empty string clears all requires.
    pub new_requires: Option<String>,
    /// Full ensures clause lines, e.g. `"ensures result >= 0"`. Empty string clears.
    pub new_ensures: Option<String>,
}

/// Locate structural spans for `fn name(...) -> T requires... { ... }`.
#[derive(Debug, Clone)]
struct FnLayout {
    /// Index of `fn` keyword (or `pub fn` — we still search `fn name`).
    paren_open: usize,
    paren_close: usize,
    /// End (exclusive) of return type text (start of contracts/body).
    ret_end: usize,
    /// Start of first `requires`/`ensures` keyword after header, or body `{` if none.
    contracts_start: usize,
    body_open: usize,
    body_close: usize,
}

pub fn apply_patch(file_path: &Path, patch_json: &str) -> Result<()> {
    let patch: AstPatch = serde_json::from_str(patch_json)
        .map_err(|e| anyhow!("Invalid patch JSON: {}", e))?;

    let has_edit = patch.new_body.is_some()
        || patch.new_body_expr.is_some()
        || patch.new_params.is_some()
        || patch.new_return_type.is_some()
        || patch.new_requires.is_some()
        || patch.new_ensures.is_some();
    if !has_edit {
        return Err(anyhow!(
            "Patch for '{}' must include at least one of: new_body, new_body_expr, \
             new_params, new_return_type, new_requires, new_ensures",
            patch.target_function
        ));
    }

    let code = fs::read_to_string(file_path)?;
    let mut lexer = Lexer::new(&code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    let exists = program.items.iter().any(|item| {
        matches!(item, crate::ast::Item::Function(f) if f.name == patch.target_function)
    });
    if !exists {
        return Err(anyhow!(
            "Target function '{}' not found in {}",
            patch.target_function,
            file_path.display()
        ));
    }

    let layout = find_fn_layout(&code, &patch.target_function)?;
    let mut new_code = code.clone();

    // Apply header edits first (params / return / contracts), then body.
    // Order: contracts (before body brace), return type, params — work back-to-front
    // so indices stay valid... Actually we rebuild from layout each time from latest string.
    if patch.new_params.is_some()
        || patch.new_return_type.is_some()
        || patch.new_requires.is_some()
        || patch.new_ensures.is_some()
    {
        new_code = apply_header_edits(&new_code, &patch)?;
    }

    if patch.new_body.is_some() || patch.new_body_expr.is_some() {
        let body_src = resolve_body(&patch)?;
        // Validate body alone parses.
        let probe = format!("fn __patch_probe() {{\n{}\n}}\n", body_src);
        let mut probe_lex = Lexer::new(&probe);
        let probe_tokens = probe_lex
            .tokenize()
            .map_err(|e| anyhow!("Patch body lex error: {}", e))?;
        let mut probe_parser = Parser::new(probe_tokens);
        probe_parser
            .parse_program()
            .map_err(|e| anyhow!("Patch body parse error: {}", e))?;
        new_code = replace_function_body(&new_code, &patch.target_function, &body_src)?;
        let _ = layout; // layout from original; body replace re-finds
    }

    validate_and_write(file_path, &new_code, &patch.target_function)?;
    Ok(())
}

fn resolve_body(patch: &AstPatch) -> Result<String> {
    if let Some(b) = &patch.new_body {
        Ok(b.clone())
    } else if let Some(expr) = &patch.new_body_expr {
        if expr.trim().ends_with(';') {
            Ok(format!(
                "return {};",
                expr.trim().trim_end_matches(';')
            ))
        } else {
            Ok(format!("return {};", expr.trim()))
        }
    } else {
        Err(anyhow!("internal: resolve_body called without body fields"))
    }
}

fn apply_header_edits(source: &str, patch: &AstPatch) -> Result<String> {
    let layout = find_fn_layout(source, &patch.target_function)?;
    let mut out = source.to_string();

    // 1) Params: replace inside ( ... )
    if let Some(params) = &patch.new_params {
        let open = layout.paren_open;
        let close = layout.paren_close;
        let mut s = String::new();
        s.push_str(&out[..open + 1]);
        s.push_str(params.trim());
        s.push_str(&out[close..]);
        out = s;
    }

    // Re-find layout after params change
    let layout = find_fn_layout(&out, &patch.target_function)?;

    // 2) Return type
    if let Some(ret) = &patch.new_return_type {
        out = replace_return_type(&out, &layout, ret.trim())?;
    }

    let layout = find_fn_layout(&out, &patch.target_function)?;

    // 3) Contracts (requires/ensures) between ret_end and body_open
    if patch.new_requires.is_some() || patch.new_ensures.is_some() {
        out = replace_contracts(&out, &layout, patch)?;
    }

    Ok(out)
}

fn replace_return_type(source: &str, layout: &FnLayout, new_ret: &str) -> Result<String> {
    // After `)`, skip ws. If `->` exists, replace type; else insert ` -> Type` before contracts/body.
    let after_paren = layout.paren_close + 1;
    let bytes = source.as_bytes();
    let mut i = after_paren;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    if i + 1 < bytes.len() && &source[i..i + 2] == "->" {
        // Replace from after `->` through layout.ret_end (exclusive of contracts)
        let type_start = {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            j
        };
        let type_end = layout.ret_end;
        let mut s = String::new();
        s.push_str(&source[..type_start]);
        s.push_str(new_ret);
        // preserve a trailing space if contracts/body follow immediately
        if type_end < source.len() && !source[type_end..].starts_with(|c: char| c.is_whitespace()) {
            s.push(' ');
        }
        s.push_str(&source[type_end..]);
        Ok(s)
    } else {
        // Insert ` -> Type` before contracts / body
        let insert_at = layout.ret_end.min(layout.contracts_start);
        let mut s = String::new();
        s.push_str(&source[..insert_at]);
        // trim trailing space then add clean
        while s.ends_with(|c: char| c == ' ' || c == '\t') {
            s.pop();
        }
        s.push_str(" -> ");
        s.push_str(new_ret);
        s.push(' ');
        s.push_str(&source[insert_at..]);
        Ok(s)
    }
}
