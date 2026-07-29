// ===================================================================
// openOODA Energy-Maneuverability telemetry (honest, measured only)
//
// Boyd / E-M is the *design lens* (raise V, cut D, cut W). This report does
// **not** fabricate T/D forces or 82.4% scores. It reports measured wall-clock
// µs (latency), source weight W, and derived throughput V = W/time.
// ===================================================================
use serde::{Deserialize, Serialize};

/// Measured analysis path for one file — no hardcoded savings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmReport {
    pub file: String,
    /// Source weight W (bytes).
    pub source_bytes: usize,
    pub parse_us: u128,
    pub capability_us: u128,
    pub typecheck_us: u128,
    /// parse + capability + typecheck
    pub total_us: u128,
    /// V proxy: source bytes processed per second on the analyze path.
    pub analyze_throughput_bps: f64,
    /// Inverse latency (Hz-ish): 1e6/total_us. Higher = faster analyze pass.
    /// Not full Boyd Ps = V·(T−D)/W (needs real T/D instrumentation).
    pub inverse_latency_per_sec: f64,
    /// True if capability or typecheck failed (rework loop until fixed).
    pub check_failed: bool,
}

impl EmReport {
    pub fn from_measured(
        file: impl Into<String>,
        source_bytes: usize,
        parse_us: u128,
        capability_us: u128,
        typecheck_us: u128,
        check_failed: bool,
    ) -> Self {
        let total_us = parse_us
            .saturating_add(capability_us)
            .saturating_add(typecheck_us)
            .max(1);
        let analyze_throughput_bps =
            (source_bytes as f64) * 1_000_000.0 / (total_us as f64);
        let inverse_latency_per_sec = 1_000_000.0 / (total_us as f64);
        Self {
            file: file.into(),
            source_bytes,
            parse_us,
            capability_us,
            typecheck_us,
            total_us,
            analyze_throughput_bps,
            inverse_latency_per_sec,
            check_failed,
        }
    }

    pub fn display_summary(&self) -> String {
        format!(
            "[openOODA E-M] measured analysis of {}\n\
               W (source weight):      {} bytes\n\
               load+parse:             {} µs\n\
               capability check:       {} µs\n\
               typecheck:              {} µs\n\
               total (latency):        {} µs\n\
               V (analyze throughput): {:.0} B/s\n\
               1/latency:              {:.2} /s{}\n\
               (Measured clocks only — not invented drag-% or Boyd Ps scores.)",
            self.file,
            self.source_bytes,
            self.parse_us,
            self.capability_us,
            self.typecheck_us,
            self.total_us,
            self.analyze_throughput_bps,
            self.inverse_latency_per_sec,
            if self.check_failed {
                "\n   check_failed:           true (rework until green — D > 0)"
            } else {
                ""
            }
        )
    }
}

/// Raw measured durations (for diagnostics / callers that only need clocks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasuredTimings {
    pub parse_us: u128,
    pub check_us: u128,
}

impl MeasuredTimings {
    pub fn new(parse_us: u128, check_us: u128) -> Self {
        Self { parse_us, check_us }
    }

    pub fn total_us(self) -> u128 {
        self.parse_us.saturating_add(self.check_us)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_timings_are_raw_not_inflated() {
        let t = MeasuredTimings::new(500, 300);
        assert_eq!(t.parse_us, 500);
        assert_eq!(t.check_us, 300);
        assert_eq!(t.total_us(), 800);
    }

    #[test]
    fn em_report_has_no_magic_constants() {
        let r = EmReport::from_measured("t.oo", 1000, 200, 50, 50, false);
        assert_eq!(r.total_us, 300);
        assert!((r.analyze_throughput_bps - (1000.0 * 1e6 / 300.0)).abs() < 1.0);
        let s = r.display_summary();
        assert!(!s.contains("82.4"), "no fake savings: {}", s);
        assert!(!s.contains("OPTIMAL MANEUVERABILITY"), "no marketing floor: {}", s);
        assert!(s.contains("measured"), "must claim measured: {}", s);
    }

    #[test]
    fn em_report_marks_check_failed() {
        let r = EmReport::from_measured("bad.oo", 10, 1, 1, 1, true);
        assert!(r.check_failed);
        assert!(r.display_summary().contains("check_failed"));
    }
}
