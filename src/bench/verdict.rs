// openOODA Empirical Claim Verification Suite — verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Verified,
    TrapFired,
    NotApplicable,
    Failed,
}

impl Verdict {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Verdict::Verified => "VERIFIED",
            Verdict::TrapFired => "TRAP FIRED",
            Verdict::NotApplicable => "NOT APPLICABLE",
            Verdict::Failed => "FAILED",
        }
    }
}
