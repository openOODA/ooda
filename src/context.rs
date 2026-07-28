// ===================================================================
// openOODA context builder — real outline-based micro-context
// ===================================================================
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;

use crate::lexer::Lexer;
use crate::outline;
use crate::parser::Parser;
use crate::reflect;

pub struct ContextEngine;

impl ContextEngine {
    pub fn build_micro_context(file_path: &str, symbol: &str, vram_tier: &str) -> Result<String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err(anyhow!("context: file not found: {}", file_path));
        }
        let code = fs::read_to_string(path)?;
        let mut lexer = Lexer::new(&code);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program()?;

        let token_budget: usize = match vram_tier.to_lowercase().as_str() {
            "8gb" | "small" => 180,
            "16gb" | "medium" => 800,
            "frontier" | "cloud" | "pro" => 8000,
            _ => 180,
        };

        // Prefer symbol-specific reflection; fall back to full module outline.
        let body = match reflect::reflect_symbol(&program, symbol) {
            Ok(meta) => meta,
            Err(_) => outline::generate_outline(&program),
        };

        // Truncate to approximate token budget (~4 chars/token).
        let max_chars = token_budget.saturating_mul(4);
        let truncated = if body.chars().count() > max_chars {
            let t: String = body.chars().take(max_chars).collect();
            format!("{}…", t)
        } else {
            body
        };

        Ok(format!(
            "{{\n  \"target_symbol\": {},\n  \"file\": {},\n  \"tier\": {},\n  \"token_budget\": {},\n  \"context\": {}\n}}\n",
            serde_json::to_string(symbol).unwrap_or_else(|_| format!("\"{}\"", symbol)),
            serde_json::to_string(file_path).unwrap_or_else(|_| format!("\"{}\"", file_path)),
            serde_json::to_string(vram_tier).unwrap_or_else(|_| format!("\"{}\"", vram_tier)),
            token_budget,
            serde_json::to_string(&truncated).unwrap_or_else(|_| "\"\"".into()),
        ))
    }
}
