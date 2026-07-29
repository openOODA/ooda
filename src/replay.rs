// ===================================================================
// openOODA replay debugger — honest alpha gate
// ===================================================================
use anyhow::{bail, Result};

pub struct ReplayEngine;

impl ReplayEngine {
    pub fn replay_execution(file_path: &str, test_name: &str) -> Result<()> {
        println!("⏳ [Replay Engine] Initializing time-travel trace for {} in {}", test_name, file_path);
        let program = crate::loader::load_program(&std::path::PathBuf::from(file_path))?;
        let mut interp = crate::eval::Interpreter::new(program);
        match interp.call_function(test_name, vec![], &mut std::collections::HashMap::new()) {
            Ok(val) => {
                println!("✅ [Replay Complete] Target returned: {:?}", val);
                Ok(())
            }
            Err(e) => {
                println!("❌ [Replay Fault] Execution diverged: {}", e);
                bail!("Replay fault: {}", e);
            }
        }
    }
}
