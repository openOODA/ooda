//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn question_mark_void_fn_fails_check() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_qv_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("q.oo");
    std::fs::write(
        &path,
        r#"
pub fn f() -> Result[Int, String] { return Ok(1); }
pub fn main() {
    let x = f()?;
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "void main cannot use ?");
    let err = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(err.contains("`?`") || err.contains("Result"), "{}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn question_mark_err_propagates_not_add() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_qe_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("q.oo");
    std::fs::write(
        &path,
        r#"
pub fn fail() -> Result[Int, String] { return Err("nope"); }
pub fn g() -> Result[Int, String] {
    let x = fail()?;
    return Ok(x + 1);
}
pub fn main() {
    match g() {
        Ok(v) => println(v),
        Err(e) => println(e),
    }
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "should print nope not crash: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("nope"), "stdout={}", stdout);
    assert!(!stdout.contains("Invalid binary"), "must not try Err+1");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_refuses_try_operator() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_try_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("t.oo");
    std::fs::write(
        &path,
        r#"
pub fn f() -> Result[Int, String] { return Ok(1); }
pub fn g() -> Result[Int, String] {
    let x = f()?;
    return Ok(x);
}
pub fn main() {
    match g() {
        Ok(v) => println(v),
        Err(e) => println(e),
    }
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "C must refuse ?");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(err.contains("try-operator") || err.contains("`?`"), "{}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_path_exists_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_exists_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(fs: &FsCap) {
    if fs.path_exists("/tmp") {
        println("exists");
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
        "C must lower sealed .path_exists: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("exists"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_interpreter_handles_read_write_file_methods() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_run_rw_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let target_file = dir.join("test.txt");
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        format!(
            r#"
pub fn main(fs: &FsCap) {{
    let res = fs.write_file("{}", "hello ooda");
    if res.is_ok {{
        let r = fs.read_file("{}");
        match r {{
            Ok(content) => println(content),
            Err(e) => println(e),
        }}
    }}
}}
"#,
            target_file.display(),
            target_file.display()
        ),
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "run must handle .write_file / .read_file: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("hello ooda"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_file_size_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_sz_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(fs: &FsCap) {
    let sz = fs.file_size("/etc/hosts");
    if sz > 0 {
        println(sz);
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
        "C must lower sealed .file_size: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(!stdout.trim().is_empty(), "expected size print, stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

