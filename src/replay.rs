// ===================================================================
// openOODA replay debugger — honest alpha gate
// ===================================================================
use anyhow::{bail, Result};

pub struct ReplayEngine;

impl ReplayEngine {
    pub fn replay_execution(file_path: &str, test_name: &str) -> Result<()> {
        bail!(
            "ooda replay is not implemented in this alpha (refused to print a fake trace for '{}' / '{}'). \
             Use `ooda test` and `ooda run` for real execution feedback.",
            file_path,
            test_name
        );
    }
}
