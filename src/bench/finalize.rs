// Suite finalize + summary
use std::io::Write;
use anyhow::{anyhow, Result};
use super::verdict::Verdict;

pub(crate) fn finalize(out: &mut std::io::Stdout, proofs: &[(usize, Verdict)]) -> Result<()> {
    writeln!(out, "===================================================================")?;
    writeln!(out, " Per-proof verdict summary:")?;
    let mut failed = 0usize;
    let mut traps = 0usize;
    let mut na = 0usize;
    let mut ok = 0usize;
    for (n, v) in proofs {
        let tag = match v {
            Verdict::Verified => "PASS",
            Verdict::TrapFired => "TRAP",
            Verdict::NotApplicable => "N/A",
            Verdict::Failed => "FAIL",
        };
        writeln!(out, "   PROOF {}: {}", n, tag)?;
        match v {
            Verdict::Verified => ok += 1,
            Verdict::TrapFired => traps += 1,
            Verdict::NotApplicable => na += 1,
            Verdict::Failed => failed += 1,
        }
    }
    writeln!(out, "===================================================================")?;
    writeln!(
        out,
        " Totals: {} verified, {} trap-fired, {} n/a, {} failed",
        ok, traps, na, failed
    )?;
    if failed == 0 {
        writeln!(out, "✓ All applicable proofs verified (or correctly fired as traps).")?;
        writeln!(out, "===================================================================\n")?;
        Ok(())
    } else {
        writeln!(
            out,
            "❌ {} proof(s) failed. See above for details.",
            failed
        )?;
        writeln!(out, "===================================================================\n")?;
        Err(anyhow!(
            "ooda bench: {} of {} proofs failed",
            failed,
            proofs.len()
        ))
    }
}

#[cfg(test)]


mod tests {
    use super::*;

    #[test]
    fn verdict_labels_are_stable() {
        assert_eq!(Verdict::Verified.label(), "VERIFIED");
        assert_eq!(Verdict::TrapFired.label(), "TRAP FIRED");
        assert_eq!(Verdict::NotApplicable.label(), "NOT APPLICABLE");
        assert_eq!(Verdict::Failed.label(), "FAILED");
    }
}
