// ===================================================================
// openOODA edition migrator — honest alpha gate
// ===================================================================
use anyhow::{bail, Result};

pub struct MigrationEngine;

impl MigrationEngine {
    pub fn migrate_codebase(file_path: &str, target_edition: &str) -> Result<()> {
        bail!(
            "ooda migrate is not implemented in this alpha (refused to claim compliance for '{}' → edition {}). \
             No AST codemods are applied. Re-run after a real migrator ships.",
            file_path,
            target_edition
        );
    }
}
