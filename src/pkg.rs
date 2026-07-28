// ===================================================================
// openOODA package manager (honest alpha)
// ===================================================================
use anyhow::{bail, Result};
use std::fs;

pub struct PackageManager;

impl PackageManager {
    pub fn init(project_name: &str) -> Result<()> {
        if project_name.trim().is_empty() {
            bail!("ooda pkg --init requires a non-empty project name");
        }
        let manifest = format!(
            "{{\n  \"name\": \"{}\",\n  \"version\": \"0.1.0-alpha\",\n  \"dependencies\": {{}}\n}}\n",
            project_name
        );
        fs::write("ooda.json", manifest)?;
        if !std::path::Path::new("ooda.lock").exists() {
            fs::write(
                "ooda.lock",
                "# openOODA lockfile — populated when real installs are implemented\n",
            )?;
        }
        println!(
            "Initialized ooda.json for project '{}' (dependency install is not implemented in this alpha).",
            project_name
        );
        Ok(())
    }

    pub fn install(repo: &str) -> Result<()> {
        bail!(
            "ooda pkg --install is not implemented in this alpha (refused to fake a download of '{}'). \
             Create a local path dependency manually or wait for a real resolver. \
             Tip: use `ooda pkg --init <name>` to write an empty ooda.json only.",
            repo
        );
    }
}
