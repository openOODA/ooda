/// Real parse + capability + type-check timings. Wired into
/// `--json-errors` so AI agents see measured µs, not "honest
/// theater" hardcoded numbers.
struct AnalyzeTimings {
    parse_us: u128,
    capability_us: u128,
    typecheck_us: u128,
}

impl AnalyzeTimings {
    fn attach(self, d: AiDiagnostic) -> AiDiagnostic {
        d.with_timings(
            self.parse_us,
            self.capability_us.saturating_add(self.typecheck_us),
        )
    }
}
