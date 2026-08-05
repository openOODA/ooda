

/// Locate `libooda.a` from cargo target dir (release preferred).
fn find_ooda_staticlib_dir() -> Option<std::path::PathBuf> {
    let mut candidates = vec![
        std::path::PathBuf::from("target/release"),
        std::path::PathBuf::from("target/debug"),
    ];
    // Crate-relative targets (works regardless of host home path)
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(manifest.join("target/release"));
    candidates.push(manifest.join("target/debug"));
    for c in candidates {
        if c.join("libooda.a").exists() {
            return Some(c);
        }
    }
    None
}


fn c_escape_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}


fn which_gcc() -> Result<String> {
    for t in ["gcc", "cc"] {
        if Command::new(t).arg("--version").output().map(|o| o.status.success()).unwrap_or(false) {
            return Ok(t.into());
        }
    }
    bail!("no gcc/cc in PATH for CHS C backend native link")
}


fn dirs_tmp() -> String {
    if let Ok(h) = std::env::var("HOME") {
        format!("{}/.cache/ooda-tmp", h)
    } else {
        "/var/tmp".into()
    }
}

