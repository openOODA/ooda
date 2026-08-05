//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn json_errors_method_write_file_arity() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_mwf_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("mwf.oo");
    std::fs::write(
        &path,
        r#"
pub fn bad(fs: &FsCap) {
    let r = fs.write_file("/tmp/x");
    match r { Ok(_) => 0, Err(_) => 1 };
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "method arity must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    let msg = v["message"].as_str().unwrap_or("");
    assert!(
        msg.contains(".write_file") && msg.contains("expects 3"),
        "msg: {}",
        msg
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_patch_json_reports_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_pj_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("p.oo");
    std::fs::write(
        &path,
        "pub fn add(a: Int, b: Int) -> Int {\n    return a + b;\n}\n",
    )
    .unwrap();
    let diff = r#"{"target_function":"add","new_body":"return a * b;"}"#;
    let out = std::process::Command::new(bin)
        .args([
            "patch",
            path.to_str().unwrap(),
            "--diff",
            diff,
            "--json",
        ])
        .output()
        .expect("spawn patch --json");
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stdout)
    });
    assert_eq!(v["ok"].as_bool(), Some(true));
    assert_eq!(v["changed"].as_bool(), Some(true));
    assert_eq!(v["target_function"].as_str(), Some("add"));
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("a * b"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_list_elem_mismatch_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_list_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("list.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let xs = list_new();
    let ys = list_push(xs, 1);
    let zs = list_push(ys, "a");
    println(list_len(zs));
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "mixed list elements must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "list elem fix: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("list_elem") || diff.contains("List"),
        "codemod: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_assign_type_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_asg_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("asg.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut x = 1;
    x = "hi";
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
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "assign type: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("assign_type") && diff.contains("x"),
        "codemod: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_context_nests_reflection_json() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/hello.oo");
    let out = std::process::Command::new(bin)
        .args(["context", example, "greet", "--tier", "8gb"])
        .output()
        .expect("spawn context");
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Strip banner line if present
    let json_start = stdout.find('{').expect(&stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout[json_start..]).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stdout)
    });
    assert_eq!(v["target_symbol"].as_str(), Some("greet"));
    assert!(
        v["context"].is_object(),
        "context must be nested object (not escaped string): {}",
        stdout
    );
    assert_eq!(v["context"]["symbol"].as_str(), Some("greet"));
}

#[test]
fn json_errors_const_char_at_oob_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_char_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("c.oo");
    std::fs::write(
        &path,
        "pub fn main() {\n    let c = char_at(\"hi\", 99);\n    println(c);\n}\n",
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert!(
        v["message"].as_str().unwrap_or("").contains("out of bounds"),
        "{}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

