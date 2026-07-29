// ===================================================================
// openOODA context builder — real outline-based micro-context
// ===================================================================
use anyhow::{anyhow, Result};
use serde_json::json;
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

        // Nested JSON object when reflection is JSON — avoid double-encoded
        // string payload (cuts AI context weight W without losing structure).
        let context_val: serde_json::Value = serde_json::from_str(&truncated)
            .unwrap_or_else(|_| serde_json::Value::String(truncated));

        let payload = json!({
            "target_symbol": symbol,
            "file": file_path,
            "tier": vram_tier,
            "token_budget": token_budget,
            "context": context_val,
        });
        Ok(format!("{}\n", serde_json::to_string_pretty(&payload)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn context_nests_reflection_object_not_escaped_string() {
        let dir = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".cache")
            .join(format!("ooda-ctx-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("t.oo");
        let mut f = fs::File::create(&path).unwrap();
        write!(
            f,
            "pub fn greet(name: String) -> String\n    requires name.len() > 0\n{{\n    return \"hi\";\n}}\n"
        )
        .unwrap();
        let out = ContextEngine::build_micro_context(path.to_str().unwrap(), "greet", "8gb")
            .expect("context");
        let v: serde_json::Value = serde_json::from_str(out.trim()).expect(&out);
        assert_eq!(v["target_symbol"], "greet");
        assert_eq!(v["token_budget"], 180);
        // Nested object — not a string of JSON.
        assert!(
            v["context"].is_object(),
            "context must be nested object, got: {}",
            out
        );
        assert_eq!(v["context"]["symbol"], "greet");
        assert!(
            v["context"]["preconditions"].is_array(),
            "preconditions: {}",
            out
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
