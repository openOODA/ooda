// ===================================================================
// openOODA Dynamic LLM Auto-Scaler Engine (ooda context)
// Automatically scales prompt payload context from 8GB VRAM to Frontier Cloud LLMs
// ===================================================================
use anyhow::Result;

pub struct ContextEngine;

impl ContextEngine {
    pub fn build_micro_context(file_path: &str, symbol: &str, vram_tier: &str) -> Result<String> {
        let (token_budget, mode) = match vram_tier.to_lowercase().as_str() {
            "8gb" | "small"   => (180, "Ultra-Compact Micro-Context (8GB VRAM Local 7B/8B LLM)"),
            "16gb" | "medium" => (800, "Balanced Context (16GB-24GB VRAM 32B/70B LLM)"),
            "frontier" | "cloud" | "pro" => (8000, "Full Reflection Context (Frontier Cloud LLM 128k-2M Window)"),
            _ => (180, "Ultra-Compact Micro-Context (Default 8GB VRAM Target)"),
        };

        let payload = format!(
            "{{\n  \"target_symbol\": \"{}\",\n  \"file\": \"{}\",\n  \"auto_scale_tier\": \"{}\",\n  \"vram_mode\": \"{}\",\n  \"token_budget\": {},\n  \"active_capabilities\": [\"&NetCap\", \"&FsCap\"]\n}}\n",
            symbol, file_path, vram_tier, mode, token_budget
        );
        Ok(payload)
    }
}
