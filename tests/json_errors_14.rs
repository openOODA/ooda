//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn build_c_lowers_str_slice_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_slice_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    println("hello".str_slice(0, 2));
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "C must lower .str_slice: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("he"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}


#[test]
fn json_errors_unknown_method_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_um_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("u.oo");
    std::fs::write(&path, "pub fn main() {\n    let x = 1.foo();\n    println(x);\n}\n").unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect(&stderr);
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "{}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(diff.contains("unknown_method"), "{}", diff);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn wrong_kind_free_write_file_net_only() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wkf_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("w.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(net: &NetCap) {
    let r = write_file(net, "/tmp/x", "y");
    println(r);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("wrong-kind") && stderr.contains("NetCap") && stderr.contains("FsCap"),
        "{}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nested_return_refinement_fails_check() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_nr_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("n.oo");
    std::fs::write(
        &path,
        r#"
type Port = Int[1..10];
pub fn f(b: Bool) -> Port {
    if b { return 99; }
    return 1;
}
pub fn main() { println(f(true)); }
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "nested return OOB must fail check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(err.contains("RefinementTypeViolation"), "{}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_sys_exec_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_sys_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(sys: &SysCap) -> Int {
    let code = sys.sys_exec("true");
    return code;
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "C must lower sealed .sys_exec: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success(), "sys_exec binary failed");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_contains_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_contains_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let s = "hello world";
    if s.contains("world") {
        println("ok");
    }
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "C must lower .contains: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("ok"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_undefined_var_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_uv_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("u.oo");
    std::fs::write(&path, "pub fn main() {\n    println(missing);\n}\n").unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect(&stderr);
    assert_eq!(v["suggested_fix"]["applicability"].as_str(), Some("patch"));
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(diff.contains("undefined_var") && diff.contains("missing"), "{}", diff);
    let _ = std::fs::remove_dir_all(&dir);
}

