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

    /// Pin a path (local or remote) into `ooda.lock` by content/metadata hash.
    /// Uses `curl` and `tar` for remote URLs to avoid heavy rust dependencies (E-M constraint).
    pub fn install(repo: &str) -> Result<()> {
        let is_remote = repo.starts_with("http://") || repo.starts_with("https://");

        if repo.starts_with("git@") {
            bail!("ooda pkg --install: git@ SSH URLs not supported. Use https:// tarballs.");
        }

        let p_owned;
        if is_remote {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(repo.as_bytes());
            let hash = format!("{:x}", hasher.finalize());
            
            let cache_dir = std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                .join(".cache")
                .join("ooda")
                .join("pkg")
                .join(&hash);

            if !cache_dir.exists() {
                fs::create_dir_all(&cache_dir)?;
                let tarball = cache_dir.join("pkg.tar.gz");
                let extract_dir = cache_dir.join("tree");
                fs::create_dir_all(&extract_dir)?;

                println!("Downloading {}...", repo);
                let status = std::process::Command::new("curl")
                    .args(["-fsSL", repo, "-o", tarball.to_str().unwrap()])
                    .status()?;
                if !status.success() {
                    bail!("Failed to download package from {}", repo);
                }

                let extract_status = std::process::Command::new("tar")
                    .args(["-xzf", tarball.to_str().unwrap(), "-C", extract_dir.to_str().unwrap()])
                    .status()?;
                if !extract_status.success() {
                    bail!("Failed to extract package from {}", repo);
                }
            }
            
            let extract_dir = cache_dir.join("tree");
            // If the tarball contains a single root folder, we use that folder.
            let mut pkg_root = extract_dir.clone();
            if let Ok(entries) = fs::read_dir(&extract_dir) {
                let entries: Vec<_> = entries.filter_map(Result::ok).collect();
                if entries.len() == 1 {
                    if let Ok(file_type) = entries[0].file_type() {
                        if file_type.is_dir() {
                            pkg_root = entries[0].path();
                        }
                    }
                }
            }
            
            p_owned = pkg_root;
        } else {
            p_owned = std::path::PathBuf::from(repo);
        }

        let p = &p_owned;
        if !p.exists() {
            bail!(
                "ooda pkg --install: local path '{}' not found.",
                p.display()
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
