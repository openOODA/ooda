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
                "# openOODA lockfile — local pins + https .tar.gz cache pins\n",
            )?;
        }
        println!(
            "Initialized ooda.json for project '{}'. \
             `ooda pkg --install` supports local paths and https://…/*.tar.gz (curl+tar). \
             No registry, no git clone, no signature verify in this alpha.",
            project_name
        );
        Ok(())
    }

    /// Install / pin:
    /// - **local path**: hash pin into `ooda.lock` only (no copy)
    /// - **https://…/*.tar.gz|*.tgz**: download with `curl`, extract with `tar` into
    ///   `~/.cache/ooda/pkg/<url-hash>/`, then pin that tree
    ///
    /// Fail-closed: git@, git clone URLs, non-tarball https, missing curl/tar.
    pub fn install(repo: &str) -> Result<()> {
        if repo.starts_with("git@")
            || repo.contains("git://")
            || repo.ends_with(".git")
            || repo.contains(".git#")
            || repo.contains(".git?")
        {
            bail!(
                "ooda pkg --install: git repositories are not supported in this alpha \
                 (refused '{}'). Use a local path or an https://…/*.tar.gz tarball.",
                repo
            );
        }

        let is_remote = repo.starts_with("http://") || repo.starts_with("https://");
        let p_owned;

        if is_remote {
            let lower = repo.to_ascii_lowercase();
            if !(lower.ends_with(".tar.gz") || lower.ends_with(".tgz")) {
                bail!(
                    "ooda pkg --install: remote install only accepts https://…/*.tar.gz or *.tgz \
                     (got '{}'). No package registry or git clone.",
                    repo
                );
            }
            if std::process::Command::new("curl")
                .arg("--version")
                .output()
                .map(|o| !o.status.success())
                .unwrap_or(true)
            {
                bail!("ooda pkg --install: `curl` not available on PATH (required for remote tarballs)");
            }
            if std::process::Command::new("tar")
                .arg("--version")
                .output()
                .map(|o| !o.status.success())
                .unwrap_or(true)
            {
                bail!("ooda pkg --install: `tar` not available on PATH (required for remote tarballs)");
            }

            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(repo.as_bytes());
            let url_hash = format!("{:x}", hasher.finalize());

            let cache_dir = std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
                .join(".cache")
                .join("ooda")
                .join("pkg")
                .join(&url_hash);

            let extract_dir = cache_dir.join("tree");
            if !extract_dir.exists() {
                fs::create_dir_all(&extract_dir)?;
                let tarball = cache_dir.join("pkg.tar.gz");
                eprintln!("ooda pkg: downloading {} …", repo);
                let status = std::process::Command::new("curl")
                    .args(["-fsSL", repo, "-o"])
                    .arg(&tarball)
                    .status()?;
                if !status.success() {
                    let _ = fs::remove_dir_all(&cache_dir);
                    bail!("ooda pkg --install: curl failed for {}", repo);
                }
                let extract_status = std::process::Command::new("tar")
                    .args(["-xzf"])
                    .arg(&tarball)
                    .arg("-C")
                    .arg(&extract_dir)
                    .status()?;
                if !extract_status.success() {
                    let _ = fs::remove_dir_all(&cache_dir);
                    bail!(
                        "ooda pkg --install: tar extract failed for {} (not a valid .tar.gz?)",
                        repo
                    );
                }
            }

            let mut pkg_root = extract_dir.clone();
            if let Ok(entries) = fs::read_dir(&extract_dir) {
                let entries: Vec<_> = entries.filter_map(Result::ok).collect();
                if entries.len() == 1 {
                    if let Ok(ft) = entries[0].file_type() {
                        if ft.is_dir() {
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
                "ooda pkg --install: path '{}' not found after resolve.",
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
                "# openOODA lockfile — local pins + https .tar.gz cache pins\n",
            )?;
        }

        let mut lock = fs::read_to_string("ooda.lock").unwrap_or_default();
        let line = format!("{}={}\n", repo, hash);
        if !lock.lines().any(|l| l.starts_with(&format!("{}=", repo))) {
            lock.push_str(&line);
            fs::write("ooda.lock", lock)?;
        }

        println!(
            "Pinned '{}' → {} (hash {}). Remote installs cache under ~/.cache/ooda/pkg/. \
             No registry resolve, no signature verify, no automatic OODA_PATH wiring.",
            repo,
            p.display(),
            hash
        );
        Ok(())
    }
}
