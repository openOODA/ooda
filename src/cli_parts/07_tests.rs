#[cfg(test)]
mod version_consistency_tests {
    /// Sentinel that prevents version drift across artifacts.
    ///
    /// Bug history: rounds 6, 7, and 8 each had to manually re-align
    /// `Cargo.toml`, `src/main.rs` (clap version), `scripts/release.sh`,
    /// `README.md`, `qa/README.md`, and `docs/index.html`. This test
    /// fails CI if any future bump forgets an artifact, locking in
    /// one canonical version per release.
    ///
    /// If you need to bump: change every string below to the new
    /// version, then commit.
    const CANONICAL_VERSION: &str = "v0.181.0-alpha";
    // For comparing against Cargo.toml which lacks the 'v'
    const CANONICAL_VERSION_NO_V: &str = "0.181.0-alpha";

    fn clap_version() -> &'static str {
        // Cli derive lives in cli_parts after main.rs modularize.
        let src = include_str!("01_types.rs");
        for line in src.lines() {
            if let Some(rest) = line.strip_prefix("#[command(version = \"") {
                if let Some(v) = rest.strip_suffix("\")]") {
                    return v;
                }
            }
        }
        panic!("could not locate `#[command(version = ...)]` in src/cli_parts/01_types.rs");
    }

    #[test]
    fn clap_version_matches_canonical() {
        assert_eq!(clap_version(), CANONICAL_VERSION_NO_V);
    }

    #[test]
    fn cargo_pkg_version_matches_canonical() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            CANONICAL_VERSION_NO_V,
            "Cargo.toml package version must match CANONICAL_VERSION_NO_V"
        );
    }

    #[test]
    fn release_sh_version_derives_from_cargo() {
        // release.sh's default VERSION reads Cargo.toml's version at
        // build time (`v${CARGO_VER}`), so any bump to Cargo.toml
        // propagates automatically. This test asserts that the
        // release script still derives from Cargo rather than a
        // hardcoded string.
        let sh = include_str!("../../scripts/release.sh");
        for line in sh.lines() {
            if line.contains("VERSION=") && line.contains("CARGO_VER") {
                return;
            }
        }
        panic!(
            "scripts/release.sh must derive its default VERSION from \
             Cargo.toml via CARGO_VER (no hardcoded version string)."
        );
    }

    #[test]
    fn readme_version_matches_canonical() {
        let readme = include_str!("../../README.md");
        for line in readme.lines() {
            if !line.starts_with("**openOODA Project**") {
                continue;
            }
            // Find the substring after "Version `".
            if let Some(idx) = line.find("Version `") {
                let rest = &line[idx + "Version `".len()..];
                if let Some(v) = rest.split('`').next() {
                    assert_eq!(v, CANONICAL_VERSION,
                        "README.md version header does not match the canonical version");
                    return;
                }
            }
            panic!("README header lacks Version-anchor: {}", line);
        }
        panic!("could not locate README version header");
    }

    #[test]
    fn install_oo_default_pin_matches_canonical() {
        // Default pin lives in layout.oo (install.oo is CLI-only after chapter split).
        let layout = include_str!("../../install/layout.oo");
        let needle = format!("\"{}\"", CANONICAL_VERSION);
        assert!(
            layout.contains(&needle),
            "install/layout.oo default OODA_VERSION pin must be {}",
            CANONICAL_VERSION
        );
    }

    #[test]
    fn bootstrap_pin_file_matches_canonical() {
        let pin = include_str!("../../install/BOOTSTRAP_PIN").trim();
        assert_eq!(
            pin, CANONICAL_VERSION,
            "install/BOOTSTRAP_PIN must match Cargo-derived canonical version \
             (sync openooda-gh-pages install defaults from this file)"
        );
    }

    /// When the monorepo sibling website is present, install entrypoints must
    /// pin the same version (stops homepage CTA thrash to stale tags).
    #[test]
    fn monorepo_site_install_pins_match_canonical_if_present() {
        let candidates = [
            "../openOODA.github.io/install",
            "../openOODA.github.io/install.sh",
        ];
        let mut saw_any = false;
        for rel in candidates {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
            if !path.is_file() {
                continue;
            }
            saw_any = true;
            let body = std::fs::read_to_string(&path).expect("read site install");
            let needle = format!("OODA_VERSION:-{}", CANONICAL_VERSION);
            assert!(
                body.contains(&needle) || body.contains(&format!("\"{}\"", CANONICAL_VERSION)),
                "{} must pin {} (found neither OODA_VERSION:-{} nor quoted pin)",
                path.display(),
                CANONICAL_VERSION,
                CANONICAL_VERSION
            );
        }
        // In monorepo checkouts this must fire; bare ooda clone alone is ok to skip.
        let _ = saw_any;
    }

    /// Docs brand README (if monorepo sibling present) must not lag the pin.
    #[test]
    fn monorepo_docs_readme_pin_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/README.md");
        if !path.is_file() {
            return;
        }
        let body = std::fs::read_to_string(&path).expect("docs README");
        assert!(
            body.contains(CANONICAL_VERSION),
            "docs/README.md must mention {} when monorepo sibling present",
            CANONICAL_VERSION
        );
    }

}
