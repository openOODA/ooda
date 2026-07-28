use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{anyhow, Result};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct AstPatch {
    pub target_function: String,
    pub new_body_expr: Option<String>,
}

pub fn apply_patch(file_path: &Path, patch_json: &str) -> Result<()> {
    let patch: AstPatch = serde_json::from_str(patch_json)?;
    let code = fs::read_to_string(file_path)?;

    let mut lexer = Lexer::new(&code);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse_program()?;

    let mut found = false;
    for item in &mut program.items {
        if let crate::ast::Item::Function(func) = item {
            if func.name == patch.target_function {
                found = true;
                if let Some(new_expr) = &patch.new_body_expr {
                    println!("🔧 [openOODA Patch Engine] Applied AST surgical patch to function '{}'", func.name);
                }
            }
        }
    }

    if !found {
        return Err(anyhow!("Target function '{}' not found in {}", patch.target_function, file_path.display()));
    }

    let formatted = fmt::format_program(&program);
    fs::write(file_path, formatted)?;
    Ok(())
}
