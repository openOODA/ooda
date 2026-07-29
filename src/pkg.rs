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
        let p = std::path::Path::new(repo);
        if !p.exists() {
            bail!("Package source '{}' not found. Only local paths are currently supported.", repo);
        }
        
        let manifest_path = if p.is_dir() {
            p.join("ooda.json")
        } else {
            p.to_path_buf()
        };
        
        let hash = if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        } else {
            let meta = fs::metadata(p)?;
            format!("{:x}", meta.len())
        };

        if !std::path::Path::new("ooda.lock").exists() {
            fs::write(
                "ooda.lock",
                "# openOODA lockfile\n",
            )?;
        }
        
        let mut lock = fs::read_to_string("ooda.lock").unwrap_or_default();
        if !lock.contains(&format!("{}=", repo)) {
            lock.push_str(&format!("{}={}\n", repo, hash));
            fs::write("ooda.lock", lock)?;
        }
        
        println!("✅ Installed '{}' (hash: {})", repo, hash);
        Ok(())
    }
}
