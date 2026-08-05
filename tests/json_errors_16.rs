//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn run_interpreter_handles_env_get_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_run_env_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(env: &EnvCap) {
    let res = env.env_get("PATH");
    if res.is_ok {
        println("env ok");
    }
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "run must handle .env_get: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("env ok"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_env_get_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_env_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(env: &EnvCap) {
    let res = env.env_get("PATH");
    if res.is_ok {
        println("env ok");
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
        "C must lower sealed .env_get: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("env ok"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn run_interpreter_handles_env_get_missing_key() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_run_env_miss_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    // Unlikely to be set in CI; if set, test still passes on is_err branch absence.
    std::fs::write(
        &path,
        r#"
pub fn main(env: &EnvCap) {
    let res = env.env_get("OODA_TEST_MISSING_ENV_KEY_9f3a2c1b");
    if res.is_err {
        println("missing ok");
    }
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "run must handle missing .env_get: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("missing ok"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_result_is_ok_without_sealed_io() {
    // Pure Result probe (no sealed I/O) must still compile on C.
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_isok_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let r: Result[String, String] = Ok("hi");
    if r.is_ok {
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
        "C must lower Result.is_ok without sealed I/O: {}",
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
fn build_c_lowers_string_methods() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_strmeth_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let n = 42;
    let s = n.to_string();
    let s2 = "  HELLO  ".trim();
    let s3 = s2.to_lowercase();
    if s == "42" { println("ok1"); }
    if s2 == "HELLO" { println("ok2"); }
    if s3 == "hello" { println("ok3"); }
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
        "C must lower string methods: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("ok1"), "stdout={}", stdout);
    assert!(stdout.contains("ok2"), "stdout={}", stdout);
    assert!(stdout.contains("ok3"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_nested_field_assign() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_nestfld_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
type Inner = struct { n: Int };
type Outer = struct { inner: Inner };
pub fn main() {
    let mut o = Outer { inner: Inner { n: 1 } };
    o.inner.n = 42;
    println(o.inner.n);
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
        "C nested field assign: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("42"), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

