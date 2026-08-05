
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
