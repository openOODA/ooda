
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
