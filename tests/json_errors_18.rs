//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn build_c_refuses_mkdir_p_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_mkdir_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(fs: &FsCap) {
    fs.mkdir_p("/tmp/ooda_should_refuse_c");
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "C must refuse sealed .mkdir_p");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("mkdir"),
        "got: {}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn break_continue_run_and_c() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_bc_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut i = 0;
    let mut s = 0;
    while i < 10 {
        i = i + 1;
        if i == 3 { continue; }
        if i == 5 { break; }
        s = s + i;
    }
    println(s);
}
"#,
    )
    .unwrap();
    let run = std::process::Command::new(bin)
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("run");
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert!(String::from_utf8_lossy(&run.stdout).contains("7"), "stdout={}", String::from_utf8_lossy(&run.stdout));
    let build = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("build");
    assert!(build.status.success(), "C: {}", String::from_utf8_lossy(&build.stderr));
    let exe = path.with_extension("");
    let out = std::process::Command::new(&exe).output().expect("exe");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("7"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn break_outside_loop_fails_check() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_br_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(&path, "pub fn main() { break; }\n").unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let err = format!("{}{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
    assert!(err.contains("break") || err.contains("loop"), "{}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

/// R1: `.len(1)` arity fail-closed.
#[test]
fn oodac_typecheck_rejects_len_arity() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/len_arity.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let c = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(c.contains("len") || c.contains("argument") || c.contains("ERR"), "got {}", c);
}

/// R1: `.contains()` missing arg fail-closed.
#[test]
fn oodac_typecheck_rejects_contains_arity() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/contains_arity.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: known struct, unknown field fail-closed.
#[test]
fn oodac_typecheck_rejects_struct_bad_field() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/struct_bad_field.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let c = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(c.contains("field") || c.contains("Point") || c.contains("ERR"), "got {}", c);
}

/// R1: known struct field access OK.
#[test]
fn oodac_typecheck_struct_field_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/struct_field_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: multi-arg call type mismatch fail-closed.
#[test]
fn oodac_typecheck_rejects_multi_arg_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/multi_arg_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: multi-arg call OK when types match.
#[test]
fn oodac_typecheck_multi_arg_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/multi_arg_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: multi-arg `f(g(), h())` mismatch fail-closed.
#[test]
fn oodac_typecheck_rejects_multi_arg_call_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/multi_arg_call_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}


/// R1: f(p.x) when field Int and f expects String fail-closed.
#[test]
fn oodac_typecheck_rejects_field_into_call_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/field_into_call_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let c = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(c.contains("argument") || c.contains("Int") || c.contains("ERR"), "got {}", c);
}

/// R1: f(p.x) OK when types match.
