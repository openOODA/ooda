// ===================================================================
// Optional measured timings helper (not "energy savings" marketing).
// Only attach to diagnostics when callers pass real measured µs.
// ===================================================================

/// Raw measured durations — no hardcoded "82.4% savings" constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}
