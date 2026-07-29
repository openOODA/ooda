//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn json_errors_golden_capability_violation() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/unauthorized_io.oo");
    let out = std::process::Command::new(bin)
        .args(["check", example, "--json-errors"])
        .output()
        .expect("spawn ooda check");
    assert!(
        !out.status.success(),
        "unauthorized_io.oo must fail non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("--json-errors stderr is not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(
        v["error_type"].as_str(),
        Some("CapabilitySecurityViolation"),
        "diag: {}",
        stderr
    );
    assert!(
        v["line"].as_u64().unwrap_or(0) >= 1,
        "line must be set: {}",
        stderr
    );
    let msg = v["message"].as_str().unwrap_or("");
    assert!(!msg.is_empty(), "message must be non-empty");
    assert!(
        msg.contains("fetch") || msg.contains("NetCap") || msg.contains("Capability"),
        "message should mention capability/fetch: {}",
        msg
    );
    assert!(
        v["suggested_fix"].is_object(),
        "suggested_fix required for AI auto-fix: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(!diff.is_empty(), "suggested_fix.diff must be non-empty");
    assert!(
        diff.contains("rogue_fetch") || diff.contains("NetCap") || diff.contains("fetch"),
        "cap fix should name the function or cap/effect: {}",
        diff
    );
    // Cap fixes are machine-applicable ooda-patch JSON (not advisory theater).
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "cap suggested_fix must be applicability=patch: {}",
        stderr
    );
    assert!(
        diff.contains("target_function") || diff.contains('{'),
        "patch applicability should look like ooda patch JSON: {}",
        diff
    );
    // Honesty: no hardcoded E-M theater.
    assert!(
        v.get("em_savings").is_none(),
        "em_savings must not be injected as fake telemetry: {}",
        stderr
    );
    // Real measured clocks must be present on the error path.
    assert!(
        v["timings_us"].is_object(),
        "timings_us required (measured parse/check µs): {}",
        stderr
    );
    assert!(
        v["timings_us"]["parse_us"].as_u64().is_some(),
        "timings_us.parse_us missing: {}",
        stderr
    );
    assert!(
        v["timings_us"]["check_us"].as_u64().is_some(),
        "timings_us.check_us missing: {}",
        stderr
    );
}

#[test]
fn json_errors_undefined_function_names_symbol() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_undef_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("undef.oo");
    std::fs::write(
        &path,
        "pub fn main() {\n    let x = totally_missing_fn(1);\n    println(x);\n}\n",
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn ooda check");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect(&stderr);
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("totally_missing_fn"),
        "fix should name the symbol: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_wasm_refuses_requires_contracts() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wasm_req_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("req.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int
    requires a >= 0
{
    return a + b;
}
pub fn main() {
    println(add(1, 2));
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn build wasm");
    assert!(
        !out.status.success(),
        "WASM build must refuse requires/ensures"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        err.contains("contracts") || err.contains("requires"),
        "honest message required: {}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_refuses_requires_contracts() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_req_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("req.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int
    requires a >= 0
{
    return a + b;
}
pub fn main() {
    println(add(1, 2));
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn build");
    assert!(
        !out.status.success(),
        "C build must refuse requires/ensures: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        err.contains("contracts") || err.contains("requires"),
        "honest message required: {}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_em_is_measured_not_theater() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/hello.oo");
    let out = std::process::Command::new(bin)
        .args(["em", example])
        .output()
        .expect("spawn ooda em");
    assert!(out.status.success(), "hello.oo em should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("measured"), "stdout: {}", stdout);
    assert!(stdout.contains("µs") || stdout.contains("us"), "stdout: {}", stdout);
    assert!(!stdout.contains("82.4"), "no fake savings: {}", stdout);
    assert!(
        !stdout.contains("OPTIMAL MANEUVERABILITY"),
        "no marketing floor: {}",
        stdout
    );
}

#[test]
fn ooda_em_fails_nonzero_when_check_fails() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_em_fail_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("bad.oo");
    std::fs::write(
        &path,
        r#"
pub fn rogue() {
    let r = fetch("https://evil.example");
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["em", path.to_str().unwrap()])
        .output()
        .expect("spawn em");
    assert!(!out.status.success(), "em must fail non-zero when check fails");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("measured") || stdout.contains("check_failed"), "stdout: {}", stdout);
    assert!(!stdout.contains("82.4"), "no theater: {}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unfinished_cli_lsp_pkg_replay_exit_nonzero() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    for args in [
        vec!["lsp"],
        vec!["pkg", "--install", "nope"],
        vec!["replay", "x.oo", "t"],
    ] {
        let out = std::process::Command::new(bin)
            .args(&args)
            .output()
            .expect("spawn");
        assert!(
            !out.status.success(),
            "unfinished {:?} must exit non-zero",
            args
        );
        let err = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(
            err.contains("not implemented") || err.contains("refused"),
            "honest message for {:?}: {}",
            args,
            err
        );
    }
}

#[test]
fn ooda_migrate_json_reports_let_mut_fix() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_mig_json_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = 1;
    x = 2;
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["migrate", path.to_str().unwrap(), "--edition", "2026", "--json"])
        .output()
        .expect("spawn migrate --json");
    assert!(out.status.success(), "migrate --json should succeed: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stdout)
    });
    assert_eq!(v["let_mut_fixes"].as_u64(), Some(1), "stdout: {}", stdout);
    assert_eq!(v["changed"].as_bool(), Some(true));
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("let mut x"), "file rewritten: {}", after);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_em_json_is_measured_report_not_theater() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/em_demo.oo");
    let out = std::process::Command::new(bin)
        .args(["em", example, "--json"])
        .output()
        .expect("spawn ooda em --json");
    assert!(out.status.success(), "em --json should succeed: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("em --json stdout is not JSON: {}\n{}", e, stdout)
    });
    assert!(v["source_bytes"].as_u64().unwrap_or(0) > 0, "W missing: {}", stdout);
    assert!(v["parse_us"].as_u64().is_some(), "parse_us: {}", stdout);
    assert!(v["typecheck_us"].as_u64().is_some(), "typecheck_us: {}", stdout);
    assert!(v["total_us"].as_u64().unwrap_or(0) >= 1, "total_us: {}", stdout);
    assert!(v.get("em_savings").is_none(), "no theater: {}", stdout);
    assert!(!stdout.contains("82.4"), "no 82.4 theater: {}", stdout);
    assert_eq!(v["check_failed"].as_bool(), Some(false), "em_demo should check clean: {}", stdout);
}

#[test]
fn json_errors_immutable_assign_has_let_mut_patch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_imut_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("imut.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = 1;
    x = 2;
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "immutable assign must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    assert!(
        v["message"].as_str().unwrap_or("").contains("immutable")
            || v["message"].as_str().unwrap_or("").contains("let mut"),
        "msg: {}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "let-mut fix should be patch-applicable: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("let_mut") || diff.contains("let mut") || diff.contains("migrate"),
        "diff should mention let mut codemod: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_wasm_refuses_sealed_io_effects() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/chs_fs_roundtrip.oo");
    let out = std::process::Command::new(bin)
        .args(["build", example, "--target", "wasm"])
        .output()
        .expect("spawn ooda build wasm");
    assert!(!out.status.success(), "sealed FS must not compile to wasm without runtime caps");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("capability") || err.contains("read_file"),
        "expected sealed refuse, got: {}",
        err
    );
}

#[test]
fn build_c_refuses_sealed_io_effects() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/chs_fs_roundtrip.oo");
    let out = std::process::Command::new(bin)
        .args(["build", example, "--target", "c"])
        .output()
        .expect("spawn ooda build");
    assert!(!out.status.success(), "sealed FS must not compile to C without runtime caps");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("capability") || err.contains("read_file"),
        "expected sealed I/O refuse message, got: {}",
        err
    );
}
