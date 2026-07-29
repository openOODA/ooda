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
                "# openOODA lockfile — local path pins only (no network resolver)\n",
            )?;
        }
        println!(
            "Initialized ooda.json for project '{}'. \
             `ooda pkg --install <local-path>` pins a path hash into ooda.lock \
             (no dependency code copy or registry fetch in this alpha).",
            project_name
        );
        Ok(())
    }

    /// Pin a **local** path into `ooda.lock` by content/metadata hash.
    /// Does **not** download packages, copy trees, or resolve versions.
    pub fn install(repo: &str) -> Result<()> {
        if repo.starts_with("http://")
            || repo.starts_with("https://")
            || repo.starts_with("git@")
            || repo.contains("://")
        {
            bail!(
                "ooda pkg --install: remote URLs are not implemented in this alpha \
                 (refused to fake a download of '{}'). Pass an existing local path only.",
                repo
            );
        }
        let p = std::path::Path::new(repo);
        if !p.exists() {
            bail!(
                "ooda pkg --install: local path '{}' not found. \
                 Only existing local paths can be pinned (no network install).",
                repo
            );
        }

        let manifest_path = if p.is_dir() {
            p.join("ooda.json")
        } else {
            p.to_path_buf()
        };

        let hash = if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            format!("{:x}", hasher.finalize())
        } else {
            let meta = fs::metadata(p)?;
            format!("len:{:x}", meta.len())
        };

        if !std::path::Path::new("ooda.lock").exists() {
            fs::write(
                "ooda.lock",
                "# openOODA lockfile — local path pins only (no network resolver)\n",
            )?;
        }

        let mut lock = fs::read_to_string("ooda.lock").unwrap_or_default();
        let line = format!("{}={}\n", repo, hash);
        if !lock.lines().any(|l| l.starts_with(&format!("{}=", repo))) {
            lock.push_str(&line);
            fs::write("ooda.lock", lock)?;
        }

        println!(
            "Pinned local path '{}' in ooda.lock (hash {}). \
             No package code was copied or linked — path pin only.",
            repo, hash
        );
        Ok(())
    }
}
