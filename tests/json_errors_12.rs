//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn oodac_typecheck_slice_rejects_div_by_zero() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/div_by_zero.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args([
            "run",
            oodac.to_str().unwrap(),
            "--",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn oodac");
    assert!(!out.status.success(), "oodac must reject /0");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.to_lowercase().contains("zero") || combined.to_lowercase().contains("type"),
        "got: {}",
        combined
    );
}

/// R1 expand: oodac must fail-closed on undefined variables (was silent OK).
#[test]
fn oodac_typecheck_slice_rejects_undefined_var() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/undefined_var.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args([
            "run",
            oodac.to_str().unwrap(),
            "--",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn oodac");
    assert!(
        !out.status.success(),
        "oodac must reject undefined var: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("undefined") || combined.to_lowercase().contains("type"),
        "expected undefined type error, got: {}",
        combined
    );
}

/// R1 expand: oodac must fail-closed on pure lit Int+String (was silent OK).
#[test]
fn oodac_typecheck_slice_rejects_int_string_binop() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/binop_int_string.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args([
            "run",
            oodac.to_str().unwrap(),
            "--",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn oodac");
    assert!(
        !out.status.success(),
        "oodac must reject Int+String: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("ERR") && combined.to_lowercase().contains("type"),
        "expected ERR type, got: {}",
        combined
    );
}

/// R1: oodac check (real .oo) must fail-closed on annotated-let type mismatch
/// (stage-0 previously rejected; oodac used to print OK — honesty bug).
#[test]
fn oodac_typecheck_slice_rejects_let_ann_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/let_ann_mismatch.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args([
            "run",
            oodac.to_str().unwrap(),
            "--",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn oodac");
    assert!(
        !out.status.success(),
        "oodac must fail-closed on type mismatch: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("ERR") && combined.to_lowercase().contains("type"),
        "expected ERR type, got: {}",
        combined
    );
}

/// String literals → data segment + println_str; String `+` is bump-heap concat;
/// non-Add string arithmetic still fails closed (no silent pointer math).
