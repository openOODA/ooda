//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn build_wasm_string_content_eq_uses_streq() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wstreq_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("eq.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    if "aa" == "bb" {
        println(1);
    } else {
        println(0);
    }
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm streq build: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).expect("wat");
    assert!(
        wat.contains("(import \"env\" \"streq\""),
        "missing streq import:\n{}",
        wat
    );
    assert!(wat.contains("call $streq"), "missing streq call:\n{}", wat);
    // Two distinct data segments for aa/bb
    assert_eq!(
        wat.matches("(data (i32.const").count(),
        2,
        "expected two data segments:\n{}",
        wat
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// R1: `x.foo` on Int fail-closed.
#[test]
fn oodac_typecheck_rejects_field_on_int() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/field_on_int.oo");
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
    assert!(
        c.contains("unknown method") || c.contains("foo") || c.contains("ERR"),
        "got {}",
        c
    );
}

/// R1: `x.len()` on Int fail-closed.
#[test]
fn oodac_typecheck_rejects_len_on_int() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/len_on_int.oo");
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
    assert!(c.contains("len") || c.contains("String") || c.contains("ERR"), "got {}", c);
}

/// R1: `x.len()` on String OK.
#[test]
fn oodac_typecheck_len_on_string_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/len_on_string_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `g().foo` when g returns Int fail-closed.
#[test]
fn oodac_typecheck_rejects_field_on_call_int() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/field_on_call_int.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `f(g())` when g returns Int and f expects String.
#[test]
fn oodac_typecheck_rejects_call_arg_call_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_arg_call_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `f(g())` OK when types match.
#[test]
fn oodac_typecheck_call_arg_call_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/call_arg_call_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `let x = g(); f(x)` mismatch fail-closed.
#[test]
fn oodac_typecheck_rejects_call_arg_from_call_bind() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_arg_from_call_bind.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `g() + "a"` fail-closed.
#[test]
fn oodac_typecheck_rejects_call_add_string() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_add_string.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `g() && true` when g returns Int fail-closed.
#[test]
fn oodac_typecheck_rejects_call_and_int() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_and_int.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `g() < true` fail-closed (order needs numeric).
#[test]
fn oodac_typecheck_rejects_call_lt_bool() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_lt_bool.oo");
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
    assert!(
        c.contains("numeric") || c.contains("comparison") || c.contains("ERR"),
        "got {}",
        c
    );
}

/// R1: `g() < 1` when g returns String fail-closed.
#[test]
fn oodac_typecheck_rejects_call_lt_string() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_lt_string.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `g() < 2` when g returns Int OK.
