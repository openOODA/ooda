use serde::Serialize;
use std::path::Path;

/// Extract `line:col` from messages emitted by the parser, typechecker,
/// and capability checker. Returns **1-indexed** line and column.
///
/// Recognises these formats in priority order:
/// 1. `at LINE:COL`            — parser, typechecker (`Type error at 4:26: …`)
/// 2. `at line LINE, col COL`  — capability checker (`… at line 2, col 52.`)
/// 3. `line N`                 — fallback, column defaults to 1
///
/// Defaults to `(1, 1)` when no location is found (never returns 0 as source coords).
pub fn parse_loc(msg: &str) -> (usize, usize) {
    // Format 1: ` at LINE:COL ` (must not match ` at line ` first — order matters)
    if let Some(idx) = msg.find(" at ") {
        let rest = &msg[idx + 4..];
        // Skip if this is the capability form starting with "line "
        if !rest.starts_with("line ") {
            let coords: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ':')
                .collect();
            let parts: Vec<&str> = coords.split(':').collect();
            if parts.len() >= 2 {
                if let (Ok(l), Ok(c)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                    if l >= 1 && c >= 1 {
                        return (l, c);
                    }
                }
            }
        }
    }
    // Format 2: ` at line LINE, col COL `
    if let Some(idx) = msg.find(" at line ") {
        let after_line = &msg[idx + " at line ".len()..];
        let line_str: String = after_line
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(l) = line_str.parse::<usize>() {
            if let Some(comma_idx) = after_line.find(" col ") {
                let after_col = &after_line[comma_idx + " col ".len()..];
                let col_str: String = after_col
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(c) = col_str.parse::<usize>() {
                    return (l.max(1), c.max(1));
                }
            }
            return (l.max(1), 1);
        }
    }
    // Format 3: `line N`
    if let Some(idx) = msg.find("line ") {
        let rest = &msg[idx + 5..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(l) = num.parse::<usize>() {
            return (l.max(1), 1);
        }
    }
    (1, 1)
}

/// Convert 1-indexed source `(line, col)` to LSP 0-indexed `(line, character)`.
/// Never underflows: line 1 → 0, col 1 → 0.
pub fn to_lsp_position(line_1: usize, col_1: usize) -> (usize, usize) {
    (
        line_1.saturating_sub(1),
        col_1.saturating_sub(1),
    )
}

/// Map a UTF-8 byte offset to LSP 0-indexed `(line, character)`.
/// Character units are UTF-16 code units (LSP requirement); ASCII `.oo` sources
/// map 1:1. Clamps past-end offsets to EOF.
pub fn byte_offset_to_lsp(source: &str, byte: usize) -> (usize, usize) {
    let byte = byte.min(source.len());
    let mut line: usize = 0;
    let mut character: usize = 0;
    let mut i = 0usize;
    for ch in source.chars() {
        let ch_len = ch.len_utf8();
        if i + ch_len > byte {
            break;
        }
        i += ch_len;
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16();
        }
    }
    (line, character)
}

#[cfg(test)]
mod parse_loc_tests {
    use super::{parse_loc, to_lsp_position};

    #[test]
    fn extracts_at_line_col_format() {
        assert_eq!(
            parse_loc("Type error at 4:26: undefined variable 'foo'"),
            (4, 26)
        );
    }

    #[test]
    fn extracts_capability_at_line_comma_col_format() {
        let msg = "Security Capability Violation: Function 'rogue_fetch' calls sealed effectful builtin 'fetch' which requires a &NetCap parameter, but none was declared at line 2, col 52. Default-deny: grant the capability token explicitly.";
        assert_eq!(parse_loc(msg), (2, 52));
    }

    #[test]
    fn extracts_fallback_line_format() {
        assert_eq!(parse_loc("Expected token at line 7"), (7, 1));
    }

    #[test]
    fn defaults_to_one_one_when_no_match() {
        assert_eq!(parse_loc("totally unstructured error message"), (1, 1));
    }

    #[test]
    fn lsp_zero_index_mapping_no_underflow() {
        assert_eq!(to_lsp_position(1, 1), (0, 0));
        assert_eq!(to_lsp_position(4, 26), (3, 25));
        assert_eq!(to_lsp_position(0, 0), (0, 0)); // defensive
    }

    #[test]
    fn does_not_confuse_at_line_with_at_coords() {
        // " at line 2" must not be parsed as line=0 from empty coords before "line"
        let msg = "error at line 3, col 9: boom";
        assert_eq!(parse_loc(msg), (3, 9));
    }

    #[test]
    fn byte_offset_to_lsp_maps_lines() {
        let src = "ab\ncd\nef";
        // byte 0 → (0,0); after "ab\n" (3) → (1,0); after "ab\ncd\n" (6) → (2,0)
        assert_eq!(super::byte_offset_to_lsp(src, 0), (0, 0));
        assert_eq!(super::byte_offset_to_lsp(src, 3), (1, 0));
        assert_eq!(super::byte_offset_to_lsp(src, 6), (2, 0));
        assert_eq!(super::byte_offset_to_lsp(src, 7), (2, 1)); // 'e'
        // clamp past end
        assert_eq!(super::byte_offset_to_lsp(src, 999), (2, 2));
    }
}

#[derive(Debug, Serialize)]
pub struct AiDiagnostic {
    pub error_type: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<SuggestedFix>,
    /// Optional real timing telemetry (microseconds). Never inject hardcoded "savings".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings_us: Option<DiagnosticTimings>,
}

/// Measured compile-path timings for this diagnostic emission, when available.
#[derive(Debug, Serialize, Clone, Copy)]
pub struct DiagnosticTimings {
    pub parse_us: u128,
    pub check_us: u128,
}

#[derive(Debug, Serialize)]
pub struct SuggestedFix {
    pub description: String,
    pub diff: String,
    /// `patch` = machine-applicable ooda patch JSON; `advisory` = human guidance only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicability: Option<String>,
}

impl AiDiagnostic {
    pub fn new(
        error_type: impl Into<String>,
        file: &Path,
        line: usize,
        column: usize,
        message: impl Into<String>,
        explanation: impl Into<String>,
    ) -> Self {
        Self {
            error_type: error_type.into(),
            file: file.display().to_string(),
            line,
            column,
            message: message.into(),
            explanation: explanation.into(),
            suggested_fix: None,
            timings_us: None,
        }
    }

    pub fn with_fix(mut self, description: impl Into<String>, diff: impl Into<String>) -> Self {
        self.suggested_fix = Some(SuggestedFix {
            description: description.into(),
            diff: diff.into(),
            applicability: Some("advisory".into()),
        });
        self
    }

    /// Machine-applicable fix: `diff` is ooda-patch JSON (or applyable source rewrite).
    pub fn with_patch_fix(
        mut self,
        description: impl Into<String>,
        diff: impl Into<String>,
    ) -> Self {
        self.suggested_fix = Some(SuggestedFix {
            description: description.into(),
            diff: diff.into(),
            applicability: Some("patch".into()),
        });
        self
    }

    pub fn with_timings(mut self, parse_us: u128, check_us: u128) -> Self {
        self.timings_us = Some(DiagnosticTimings { parse_us, check_us });
        self
    }

    pub fn print_json(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            eprintln!("{}", json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn diagnostic_json_has_no_fake_em_savings() {
        let d = AiDiagnostic::new(
            "TypeError",
            &PathBuf::from("t.oo"),
            1,
            1,
            "msg",
            "why",
        )
        .with_fix("fix", "diff body");
        let json = serde_json::to_string(&d).unwrap();
        assert!(
            !json.contains("em_savings") && !json.contains("82.4"),
            "must not emit hardcoded E-M theater: {}",
            json
        );
        assert!(json.contains("suggested_fix"));
    }
}
