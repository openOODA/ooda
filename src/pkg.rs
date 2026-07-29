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
             Local path pin; https .tar.gz (+ optional .sha256 sidecar); git clone if git available. Set OODA_PKG_REQUIRE_SHA256=1 to fail closed without a matching sidecar for tarballs.",
            project_name
        );
        Ok(())
    }

    /// Install / pin:
    /// - **local path**: hash pin into `ooda.lock` only (no copy)
    /// - **https://…/*.tar.gz|*.tgz**: download with `curl`, extract with `tar` into
    ///   `~/.cache/ooda/pkg/<url-hash>/`, then pin that tree
    ///
    /// Fail-closed: git@, git clone URLs, non-tarball https, missing curl
    pub fn install(repo: &str) -> Result<()> {
        let is_git = repo.starts_with("git@")
            || repo.contains("git://")
            || repo.ends_with(".git")
            || repo.contains(".git#")
            || repo.contains(".git?");
        let is_remote = is_git || repo.starts_with("http://") || repo.starts_with("https://");
        let p_owned;

        if is_remote {
            let lower = repo.to_ascii_lowercase();
            if !is_git && !(lower.ends_with(".tar.gz") || lower.ends_with(".tgz")) {
                bail!(
                    "ooda pkg --install: remote install only accepts git urls or https://…/*.tar.gz|*.tgz \
                     (got '{}'). No package registry.",
                    repo
                );
            }
            if is_git {
                if std::process::Command::new("git")
                    .arg("--version")
                    .output()
                    .map(|o| !o.status.success())
                    .unwrap_or(true)
                {
                    bail!("ooda pkg --install: `git` not available on PATH (required for git URLs)");
                }
            } else {
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
                fs::create_dir_all(&cache_dir)?;
                
                if is_git {
                    eprintln!("ooda pkg: cloning {} …", repo);
                    let status = std::process::Command::new("git")
                        .args(["clone", "--depth", "1", repo, extract_dir.to_str().unwrap()])
                        .status()?;
                    if !status.success() {
                        bail!("Failed to clone package from {}", repo);
                    }
                } else {
                    fs::create_dir_all(&extract_dir)?;
                    let tarball = cache_dir.join("pkg.tar.gz");
                    eprintln!("ooda pkg: downloading {} …", repo);
                    let status = std::process::Command::new("curl")
                        .args(["-fsSL", repo, "-o", tarball.to_str().unwrap()])
                        .status()?;
                    if !status.success() {
                        let _ = fs::remove_dir_all(&cache_dir);
                        bail!("Failed to download package from {}", repo);
                    }

                    // Signed/integrity verify: optional companion URL.sha256 (or .sha256sum), .sig (GPG), or .minisig (minisign).
                    // OODA_PKG_REQUIRE_SHA256=1 requires a sidecar; default warns if absent.
                    verify_tarball_integrity(repo, &tarball)?;

                    let extract_status = std::process::Command::new("tar")
                        .args(["-xzf", tarball.to_str().unwrap(), "-C", extract_dir.to_str().unwrap()])
                        .status()?;
                    if !extract_status.success() {
                        bail!("Failed to extract package from {}", repo);
                    }
                }
            }
            
            // If the tarball or repo contains a single root folder (common for github tarballs), we use that folder.
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
             No registry resolve, no automatic OODA_PATH wiring. Tarball SHA-256 sidecar verified when present.",
            repo,
            p.display(),
            hash
        );
        Ok(())
    }
}

/// When a cryptographic sidecar is present, verification is mandatory unless
/// `OODA_PKG_ALLOW_UNSIGNED=1` explicitly opts out (fail-open escape hatch).
fn allow_unsigned() -> bool {
    std::env::var("OODA_PKG_ALLOW_UNSIGNED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Orchestrates integrity verification (minisign → GPG → SHA-256).
///
/// **Fail-closed on present signatures:** if a `.minisig` / `.sig` sidecar is
/// fetched, verification must succeed. Missing pubkey / missing `minisign` /
/// missing `gpg` does **not** silently fall through to SHA-256 unless
/// `OODA_PKG_ALLOW_UNSIGNED=1`.
fn verify_tarball_integrity(url: &str, tarball: &std::path::Path) -> Result<()> {
    // 1. Try minisign (.minisig)
    let minisig_url = format!("{}.minisig", url);
    let tmp_sig = tarball.with_extension("minisigtmp");
    if std::process::Command::new("curl")
        .args(["-fsSL", &minisig_url, "-o", tmp_sig.to_str().unwrap()])
        .status()
        .map_or(false, |s| s.success())
    {
        let pubkey = std::env::var("OODA_PKG_MINISIGN_PUBKEY").unwrap_or_default();
        if pubkey.is_empty() {
            let _ = fs::remove_file(&tmp_sig);
            if allow_unsigned() {
                eprintln!(
                    "ooda pkg: .minisig present but OODA_PKG_MINISIGN_PUBKEY unset; \
                     OODA_PKG_ALLOW_UNSIGNED=1 — continuing"
                );
            } else {
                let _ = fs::remove_file(tarball);
                bail!(
                    "ooda pkg --install: found {} but OODA_PKG_MINISIGN_PUBKEY is not set \
                     (fail-closed; set the pubkey or OODA_PKG_ALLOW_UNSIGNED=1)",
                    minisig_url
                );
            }
        } else {
            let status = std::process::Command::new("minisign")
                .args([
                    "-Vm",
                    tarball.to_str().unwrap(),
                    "-x",
                    tmp_sig.to_str().unwrap(),
                    "-P",
                    &pubkey,
                ])
                .status();
            let _ = fs::remove_file(&tmp_sig);
            match status {
                Ok(st) if st.success() => {
                    eprintln!("ooda pkg: minisign verified for {}", url);
                    return Ok(());
                }
                Ok(_) => {
                    let _ = fs::remove_file(tarball);
                    bail!(
                        "ooda pkg --install: minisign verification failed for {}",
                        url
                    );
                }
                Err(e) => {
                    let _ = fs::remove_file(tarball);
                    bail!(
                        "ooda pkg --install: minisign not runnable ({}) while .minisig present for {}",
                        e,
                        url
                    );
                }
            }
        }
        let _ = fs::remove_file(&tmp_sig);
    }

    // 2. Try GPG (.sig)
    let sig_url = format!("{}.sig", url);
    let tmp_sig = tarball.with_extension("sigtmp");
    if std::process::Command::new("curl")
        .args(["-fsSL", &sig_url, "-o", tmp_sig.to_str().unwrap()])
        .status()
        .map_or(false, |s| s.success())
    {
        let status = std::process::Command::new("gpg")
            .args([
                "--verify",
                tmp_sig.to_str().unwrap(),
                tarball.to_str().unwrap(),
            ])
            .status();
        let _ = fs::remove_file(&tmp_sig);
        match status {
            Ok(st) if st.success() => {
                eprintln!("ooda pkg: GPG verified for {}", url);
                return Ok(());
            }
            Ok(_) => {
                let _ = fs::remove_file(tarball);
                bail!("ooda pkg --install: GPG verification failed for {}", url);
            }
            Err(e) => {
                if allow_unsigned() {
                    eprintln!(
                        "ooda pkg: gpg not runnable ({}) with .sig present; \
                         OODA_PKG_ALLOW_UNSIGNED=1 — continuing",
                        e
                    );
                } else {
                    let _ = fs::remove_file(tarball);
                    bail!(
                        "ooda pkg --install: gpg not runnable ({}) while .sig present for {}",
                        e,
                        url
                    );
                }
            }
        }
    }

    // 3. Fallback to SHA-256
    verify_tarball_sha256(url, tarball)
}

/// Verify downloaded tarball against `{url}.sha256` or `{url}.sha256sum` when available.
fn verify_tarball_sha256(url: &str, tarball: &std::path::Path) -> Result<()> {
    use sha2::{Digest, Sha256};
    let require = std::env::var("OODA_PKG_REQUIRE_SHA256")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let candidates = [format!("{}.sha256", url), format!("{}.sha256sum", url)];
    let mut expected: Option<String> = None;
    for side in &candidates {
        let tmp = tarball.with_extension("sha256tmp");
        let status = std::process::Command::new("curl")
            .args(["-fsSL", side, "-o"])
            .arg(&tmp)
            .status();
        if let Ok(st) = status {
            if st.success() {
                if let Ok(body) = fs::read_to_string(&tmp) {
                    let hex = body
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_ascii_lowercase();
                    if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                        expected = Some(hex);
                        let _ = fs::remove_file(&tmp);
                        break;
                    }
                }
                let _ = fs::remove_file(&tmp);
            }
        }
    }
    let bytes = fs::read(tarball)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let actual = format!("{:x}", hasher.finalize());
    match expected {
        Some(exp) if exp == actual => {
            eprintln!("ooda pkg: SHA-256 verified for {}", url);
            Ok(())
        }
        Some(exp) => {
            let _ = fs::remove_file(tarball);
            bail!(
                "ooda pkg --install: SHA-256 mismatch for {}\n  expected {}\n  actual   {}",
                url, exp, actual
            );
        }
        None if require => {
            let _ = fs::remove_file(tarball);
            bail!(
                "ooda pkg --install: OODA_PKG_REQUIRE_SHA256=1 but no {}.sha256 sidecar found",
                url
            );
        }
        None => {
            eprintln!(
                "ooda pkg: no .sha256 sidecar for {} (set OODA_PKG_REQUIRE_SHA256=1 to require)",
                url
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{allow_unsigned, verify_tarball_sha256};

    #[test]
    fn sha256_require_without_sidecar_fails() {
        let dir = std::env::temp_dir().join(format!("ooda_sha_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let tar = dir.join("pkg.tar.gz");
        std::fs::write(&tar, b"hello-pkg-bytes").unwrap();
        // No network sidecar; REQUIRE unset → ok with warning path
        std::env::remove_var("OODA_PKG_REQUIRE_SHA256");
        assert!(verify_tarball_sha256("https://example.invalid/no-side.tar.gz", &tar).is_ok());
        // Require without sidecar fails
        std::env::set_var("OODA_PKG_REQUIRE_SHA256", "1");
        let err = verify_tarball_sha256("https://example.invalid/no-side.tar.gz", &tar).unwrap_err();
        assert!(format!("{}", err).contains("REQUIRE_SHA256") || format!("{}", err).contains("sidecar"));
        std::env::remove_var("OODA_PKG_REQUIRE_SHA256");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn allow_unsigned_env_defaults_false() {
        std::env::remove_var("OODA_PKG_ALLOW_UNSIGNED");
        assert!(!allow_unsigned());
        std::env::set_var("OODA_PKG_ALLOW_UNSIGNED", "1");
        assert!(allow_unsigned());
        std::env::remove_var("OODA_PKG_ALLOW_UNSIGNED");
    }
}
