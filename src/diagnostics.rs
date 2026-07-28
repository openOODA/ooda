use serde::Serialize;
use std::path::Path;

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
