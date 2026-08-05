//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

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
fn json_errors_missing_return_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_noret_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("noret.oo");
    std::fs::write(
        &path,
        r#"
pub fn f() -> Int {
}
pub fn main() {
    println(f());
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "missing return must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert!(
        v["message"].as_str().unwrap_or("").contains("missing return")
            || v["message"].as_str().unwrap_or("").contains("Void"),
        "msg: {}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "missing return should be patch-applicable: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("missing_return") && diff.contains("declared_return"),
        "missing_return codemod must name declared return type: {}",
        diff
    );
    assert!(
        diff.contains("Int") && diff.contains("return 0"),
        "Int missing return stub: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_unreachable_after_return_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_unreach_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("u.oo");
    std::fs::write(
        &path,
        r#"
pub fn f() -> Int {
    return 1;
    let y = 2;
}
pub fn main() {
    println(f());
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "unreachable after return must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert!(
        v["message"].as_str().unwrap_or("").contains("unreachable"),
        "msg: {}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "unreachable should be patch-applicable: {}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_patch_cli_changes_return_type() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_patch_cli_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("p.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int {
    return a + b;
}
"#,
    )
    .unwrap();
    let diff = r#"{"target_function":"add","new_return_type":"Float","new_body":"return 1.0;"}"#;
    let out = std::process::Command::new(bin)
        .args(["patch", path.to_str().unwrap(), "--diff", diff])
        .output()
        .expect("spawn patch");
    assert!(
        out.status.success(),
        "patch CLI must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(
        after.contains("-> Float") || after.contains("Float"),
        "return type patched: {}",
        after
    );
    assert!(
        after.contains("1.0"),
        "body patched: {}",
        after
    );
    // Patched program must still typecheck.
    let chk = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("check patched");
    assert!(
        chk.status.success(),
        "patched file must typecheck: {}",
        String::from_utf8_lossy(&chk.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
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
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/em_demo.oo");
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

