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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub em_savings: Option<crate::em::EmSavings>,
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
        let em = crate::em::EmSavings::calculate(400, 300, 1024, None);
        Self {
            error_type: error_type.into(),
            file: file.display().to_string(),
            line,
            column,
            message: message.into(),
            explanation: explanation.into(),
            suggested_fix: None,
            em_savings: Some(em),
        }
    }

    pub fn with_fix(mut self, description: impl Into<String>, diff: impl Into<String>) -> Self {
        self.suggested_fix = Some(SuggestedFix {
            description: description.into(),
            diff: diff.into(),
        });
        self
    }

    pub fn print_json(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            eprintln!("{}", json);
        }
    }
}
