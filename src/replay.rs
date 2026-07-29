// ===================================================================
// openOODA replay — honest alpha: re-run a named function (not time-travel)
// ===================================================================
use anyhow::{bail, Result};

pub struct ReplayEngine;

impl ReplayEngine {
    /// Re-execute `test_name` in `file_path` under the interpreter.
    /// This is **not** a time-travel debugger: no trace store, no step-back,
    /// no input recording. For real feedback use `ooda run` / `ooda test`.
    pub fn replay_execution(file_path: &str, test_name: &str) -> Result<()> {
        eprintln!(
            "ooda replay: re-running '{}' in '{}' (interpreter only; not time-travel).",
            test_name, file_path
        );
        let program = crate::loader::load_program(&std::path::PathBuf::from(file_path))?;
        crate::capabilities::CapabilityChecker::check_program(&program)?;
        crate::typecheck::TypeChecker::check_program(&program)?;
        let mut interp = crate::eval::Interpreter::new(program);
        match interp.call_function(test_name, vec![], &mut std::collections::HashMap::new()) {
            Ok(val) => {
                println!("{:?}", val);
                Ok(())
            }
            Err(e) => {
                bail!("ooda replay: execution failed: {}", e);
            }
        }
    }
}
