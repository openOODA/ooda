//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn json_errors_arg_type_mismatch_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_argty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("argty.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int {
    return a + b;
}
pub fn main() {
    let x = add("a", 2);
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "arg type mismatch must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "arg type fix should be patch: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("arg_type") && diff.contains("Int") && diff.contains("String"),
        "diff should name expected/found: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_em_json_reports_type_failed_kind() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_em_ty_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("bad.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int {
    return a + b;
}
pub fn main() {
    let x = add(1);
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["em", path.to_str().unwrap(), "--json"])
        .output()
        .expect("spawn em --json");
    assert!(!out.status.success(), "em must fail when typecheck fails");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stdout)
    });
    assert_eq!(v["check_failed"].as_bool(), Some(true));
    assert_eq!(v["type_failed"].as_bool(), Some(true));
    assert_eq!(v["cap_failed"].as_bool(), Some(false));
    assert!(v.get("em_savings").is_none(), "no theater: {}", stdout);
    assert!(!stdout.contains("82.4"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_write_file_arity_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wf_ar_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("wf.oo");
    std::fs::write(
        &path,
        r#"
pub fn bad(fs: &FsCap) {
    let r = write_file(fs, "/tmp/x");
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "write_file arity must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    let msg = v["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("write_file") && (msg.contains("expects 3") || msg.contains("found 2")),
        "msg: {}",
        msg
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "arity should be patch-applicable: {}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_em_json_reports_cap_failed_kind() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_em_cap_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("cap.oo");
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
        .args(["em", path.to_str().unwrap(), "--json"])
        .output()
        .expect("spawn em --json");
    assert!(!out.status.success(), "em must fail on cap violation");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stdout)
    });
    assert_eq!(v["check_failed"].as_bool(), Some(true));
    assert_eq!(v["cap_failed"].as_bool(), Some(true));
    // Cap fails first in em path; type may or may not run — we always run both.
    assert!(v.get("em_savings").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_return_type_mismatch_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_ret_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("ret.oo");
    std::fs::write(
        &path,
        r#"
pub fn f() -> Int {
    return "x";
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
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "return type fix: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("return_type") && diff.contains("f"),
        "codemod return_type: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

