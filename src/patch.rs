// ===================================================================
// openOODA surgical source patcher
// Replaces a named function's body with a new expression statement list.
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
    /// Example: "return a + b;"
    pub new_body: Option<String>,
    /// Legacy field accepted for older agents; treated as a single return expr if new_body unset.
    pub new_body_expr: Option<String>,
}

pub fn apply_patch(file_path: &Path, patch_json: &str) -> Result<()> {
    let patch: AstPatch = serde_json::from_str(patch_json)
        .map_err(|e| anyhow!("Invalid patch JSON: {}", e))?;

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

    let body_src = if let Some(b) = &patch.new_body {
        b.clone()
    } else if let Some(expr) = &patch.new_body_expr {
        // Treat as a return expression for backward compatibility
        if expr.trim().ends_with(';') {
            format!("return {};", expr.trim().trim_end_matches(';'))
        } else {
            format!("return {};", expr.trim())
        }
    } else {
        return Err(anyhow!(
            "Patch for '{}' must include 'new_body' (or legacy 'new_body_expr')",
            patch.target_function
        ));
    };

    // Validate the new body by wrapping it in a temporary function and parsing.
    let probe = format!("fn __patch_probe() {{\n{}\n}}\n", body_src);
    let mut probe_lex = Lexer::new(&probe);
    let probe_tokens = probe_lex
        .tokenize()
        .map_err(|e| anyhow!("Patch body lex error: {}", e))?;
    let mut probe_parser = Parser::new(probe_tokens);
    let probe_prog = probe_parser
        .parse_program()
        .map_err(|e| anyhow!("Patch body parse error: {}", e))?;
    if probe_prog.items.is_empty() {
        return Err(anyhow!("Patch body produced empty AST"));
    }

    // Textual surgical replace of the function body between the first '{' after
    // the function signature and its matching '}'. Preserves contracts/verify.
    let new_code = replace_function_body(&code, &patch.target_function, &body_src)?;

    // Atomic validation: parse, check capabilities, and check types before writing to disk
    let mut check_lexer = Lexer::new(&new_code);
    let check_tokens = check_lexer.tokenize().map_err(|e| anyhow!("Patch validation error: syntax error in patched body: {}", e))?;
    let mut check_parser = Parser::new(check_tokens);
    let check_program = check_parser.parse_program().map_err(|e| anyhow!("Patch validation error: AST parse error in patched body: {}", e))?;

    crate::capabilities::CapabilityChecker::check_program(&check_program)
        .map_err(|e| anyhow!("Patch validation error: capability violation in patched body: {}", e))?;
    crate::typecheck::TypeChecker::check_program(&check_program)
        .map_err(|e| anyhow!("Patch validation error: type error in patched body: {}", e))?;

    fs::write(file_path, &new_code)?;
    println!(
        "✂️  [openOODA Surgical Patcher] Successfully patched body of function '{}' in {}",
        patch.target_function,
        file_path.display()
    );
    Ok(())
}

fn replace_function_body(source: &str, func_name: &str, new_body: &str) -> Result<String> {
    // Find `fn name` then the opening brace of the body (not requires/ensures lines).
    let fn_pat = format!("fn {}", func_name);
    let Some(fn_idx) = source.find(&fn_pat) else {
        return Err(anyhow!("Could not locate 'fn {}' in source text", func_name));
    };

    // Scan forward for the first '{' at function body start (after header).
    let bytes = source.as_bytes();
    let mut i = fn_idx + fn_pat.len();
    let mut depth_paren = 0i32;
    while i < bytes.len() {
        match bytes[i] as char {
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '{' if depth_paren == 0 => break,
            _ => {}
        }
        i += 1;
    }
    if i >= bytes.len() {
        return Err(anyhow!("Could not find body '{{' for function '{}'", func_name));
    }
    let open = i;

    // Match braces to find closing
    let mut depth = 0i32;
    let mut j = open;
    while j < bytes.len() {
        match bytes[j] as char {
            '{' => depth += 1,
            '}' => {
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
        return Err(anyhow!("Unbalanced braces in function '{}'", func_name));
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn patch_replaces_body() {
        // Prefer $HOME over /tmp — some hosts quota tmpfs aggressively.
        let base = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let dir = base.join(".cache").join(format!("ooda-patch-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create test dir");
        let path = dir.join("t.oo");
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
        assert!(got.contains("return a * b;"));
        assert!(!got.contains("return a + b;"));
        let _ = fs::remove_dir_all(&dir);
    }
}
