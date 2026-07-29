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

fn replace_contracts(source: &str, layout: &FnLayout, patch: &AstPatch) -> Result<String> {
    // Region from contracts_start to body_open (exclusive of `{`).
    let start = layout.contracts_start;
    let end = layout.body_open;

    // Parse existing requires/ensures if only one side is being replaced.
    let existing = &source[start..end];
    let (mut reqs, mut ens) = split_contracts(existing);

    if let Some(r) = &patch.new_requires {
        reqs = r.trim().to_string();
    }
    if let Some(e) = &patch.new_ensures {
        ens = e.trim().to_string();
    }

    let mut block = String::new();
    if !reqs.is_empty() {
        for line in reqs.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with("requires") {
                block.push_str("    ");
                block.push_str(t);
                block.push('\n');
            } else {
                block.push_str("    requires ");
                block.push_str(t);
                block.push('\n');
            }
        }
    }
    if !ens.is_empty() {
        for line in ens.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with("ensures") {
                block.push_str("    ");
                block.push_str(t);
                block.push('\n');
            } else {
                block.push_str("    ensures ");
                block.push_str(t);
                block.push('\n');
            }
        }
    }

    let mut s = String::new();
    s.push_str(&source[..start]);
    if !block.is_empty() {
        // ensure newline before contracts if needed
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&block);
    } else if !s.ends_with(|c: char| c.is_whitespace()) {
        s.push(' ');
    }
    s.push_str(&source[end..]);
    Ok(s)
}

/// Split contract region into requires-blob and ensures-blob (may include keywords).
fn split_contracts(region: &str) -> (String, String) {
    let mut reqs = Vec::new();
    let mut ens = Vec::new();
    for line in region.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("requires") {
            reqs.push(t.to_string());
        } else if t.starts_with("ensures") {
            ens.push(t.to_string());
        }
    }
    (reqs.join("\n"), ens.join("\n"))
}

fn find_fn_layout(source: &str, func_name: &str) -> Result<FnLayout> {
    let fn_pat = format!("fn {}", func_name);
    let Some(fn_idx) = source.find(&fn_pat) else {
        return Err(anyhow!("Could not locate 'fn {}' in source text", func_name));
    };

    let bytes = source.as_bytes();
    let mut i = fn_idx + fn_pat.len();

    // Skip whitespace to `(`
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'(' {
        return Err(anyhow!(
            "Could not find param list '(' for function '{}'",
            func_name
        ));
    }
    let paren_open = i;

    // Match parens
    let mut depth = 0i32;
    let mut j = paren_open;
    while j < bytes.len() {
        match bytes[j] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        j += 1;
    }
    if j >= bytes.len() {
        return Err(anyhow!(
            "Unbalanced parens in parameter list for '{}'",
            func_name
        ));
    }
    let paren_close = j;

    // After `)`: optional `-> Type`, then requires/ensures, then `{`
    let mut k = paren_close + 1;
    while k < bytes.len() && bytes[k].is_ascii_whitespace() {
        k += 1;
    }

    let mut ret_end = k;

    if k + 1 < bytes.len() && &source[k..k + 2] == "->" {
        k += 2;
        while k < bytes.len() && bytes[k].is_ascii_whitespace() {
            k += 1;
        }
        // Return type: until whitespace-separated requires/ensures/{ or end
        // Types may include `Result[Int, String]`, so track brackets.
        let type_start = k;
        let mut bracket = 0i32;
        while k < bytes.len() {
            let c = bytes[k] as char;
            if c == '[' {
                bracket += 1;
                k += 1;
                continue;
            }
            if c == ']' {
                bracket -= 1;
                k += 1;
                continue;
            }
            if bracket == 0 {
                // stop at newline before requires/ensures or at `{`
                if c == '{' {
                    break;
                }
                // check keyword at line starts / after space
                if c.is_whitespace() {
                    let rest = source[k..].trim_start();
                    if rest.starts_with("requires")
                        || rest.starts_with("ensures")
                        || rest.starts_with('{')
                    {
                        // consume only the whitespace that isn't the only separator...
                        // ret_end is start of trailing whitespace before contracts
                        break;
                    }
                    // allow space inside? OODA return types don't have spaces usually
                    // but Result[Int, String] has space after comma — continue
                    k += 1;
                    continue;
                }
            }
            k += 1;
        }
        ret_end = k;
        // If we stopped on whitespace before requires, ret_end is that whitespace start — good
        let _ = type_start;
    }

    // contracts_start: skip ws after ret_end
    let mut cstart = ret_end;
    while cstart < bytes.len() && bytes[cstart].is_ascii_whitespace() {
        cstart += 1;
    }

    // Find body `{` at paren depth 0 (no nested)
    let mut b = cstart;
    while b < bytes.len() {
        if bytes[b] == b'{' {
            break;
        }
        b += 1;
    }
    if b >= bytes.len() {
        return Err(anyhow!(
            "Could not find body '{{' for function '{}'",
            func_name
        ));
    }
    let body_open = b;

    // If no requires/ensures keywords before body, contracts_start == body_open
    let contracts_start = {
        let region = &source[cstart..body_open];
        if region.contains("requires") || region.contains("ensures") {
            cstart
        } else {
            body_open
        }
    };

    // Match braces for body
    let mut depth_b = 0i32;
    let mut close = body_open;
    while close < bytes.len() {
        match bytes[close] {
            b'{' => depth_b += 1,
            b'}' => {
                depth_b -= 1;
                if depth_b == 0 {
                    break;
                }
            }
            _ => {}
        }
        close += 1;
    }
    if close >= bytes.len() {
        return Err(anyhow!("Unbalanced braces in function '{}'", func_name));
    }

    Ok(FnLayout {
        paren_open,
        paren_close,
        ret_end,
        contracts_start,
        body_open,
        body_close: close,
    })
}

fn replace_function_body(source: &str, func_name: &str, new_body: &str) -> Result<String> {
    let layout = find_fn_layout(source, func_name)?;
    let open = layout.body_open;
    let j = layout.body_close;

    let mut out = String::new();
    out.push_str(&source[..open + 1]);
    out.push('\n');
    for line in new_body.lines() {
        if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(line.trim());
            out.push('\n');
        }
    }
    out.push_str(&source[j..]);
    Ok(out)
}

fn validate_and_write(file_path: &Path, new_code: &str, target: &str) -> Result<()> {
    let mut check_lexer = Lexer::new(new_code);
    let check_tokens = check_lexer.tokenize().map_err(|e| {
        anyhow!(
            "Patch validation error: syntax error in patched source: {}",
            e
        )
    })?;
    let mut check_parser = Parser::new(check_tokens);
    let check_program = check_parser.parse_program().map_err(|e| {
        anyhow!(
            "Patch validation error: AST parse error in patched source: {}",
            e
        )
    })?;

    crate::capabilities::CapabilityChecker::check_program(&check_program).map_err(|e| {
        anyhow!(
            "Patch validation error: capability violation in patched source: {}",
            e
        )
    })?;
    crate::typecheck::TypeChecker::check_program(&check_program).map_err(|e| {
        anyhow!(
            "Patch validation error: type error in patched source: {}",
            e
        )
    })?;

    fs::write(file_path, new_code)?;
    println!(
        "✂️  [openOODA Surgical Patcher] Successfully patched function '{}' in {}",
        target,
        file_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_dir(label: &str) -> std::path::PathBuf {
        let base = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join(".cache").join(format!(
            "ooda-patch-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn patch_replaces_body() {
        let dir = test_dir("body");
        let path = dir.join("body.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return a + b;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_body":"return a * b;"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("return a * b;"), "got: {}", got);
        assert!(!got.contains("return a + b;"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_replaces_return_type() {
        let dir = test_dir("ret");
        let path = dir.join("ret.oo");
        let mut f = fs::File::create(&path).unwrap();
        // Body must match new Float return type for validation to pass.
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return 1.0;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_return_type":"Float"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("-> Float"), "got: {}", got);
        assert!(!got.contains("-> Int"), "got: {}", got);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_replaces_parameter_list() {
        let dir = test_dir("params");
        let path = dir.join("params.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return 1;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_params":"x: Int, y: Int","new_body":"return x + y;"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("fn add(x: Int, y: Int)"), "got: {}", got);
        assert!(got.contains("return x + y;"), "got: {}", got);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_sets_requires() {
        let dir = test_dir("req");
        let path = dir.join("req.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return a + b;\n}}\n"
        )
        .unwrap();
        apply_patch(
            &path,
            r#"{"target_function":"add","new_requires":"requires a >= 0"}"#,
        )
        .unwrap();
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("requires a >= 0"), "got: {}", got);
        // still typechecks / runs structurally
        assert!(got.contains("return a + b;"), "got: {}", got);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_return_type_rejects_inconsistent_body() {
        let dir = test_dir("bad");
        let path = dir.join("bad.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn add(a: Int, b: Int) -> Int {{\n    return a + b;\n}}\n"
        )
        .unwrap();
        let err = apply_patch(
            &path,
            r#"{"target_function":"add","new_return_type":"Float"}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("type error") || err.contains("Type error"),
            "expected type validation fail, got: {}",
            err
        );
        // file unchanged
        let got = fs::read_to_string(&path).unwrap();
        assert!(got.contains("-> Int"));
        let _ = fs::remove_dir_all(&dir);
    }
}
