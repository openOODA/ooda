//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn oodac_typecheck_field_into_call_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/field_into_call_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: p.x + "a" fail-closed.
#[test]
fn oodac_typecheck_rejects_field_binop_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/field_binop_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: p.x + 1 OK.
#[test]
fn oodac_typecheck_field_binop_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/field_binop_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: annotated struct let f(p.x) mismatch fail-closed.
#[test]
fn oodac_typecheck_rejects_field_into_call_ann_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/field_into_call_ann_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}


/// R1: f(o.inner.v) type mismatch fail-closed.
#[test]
fn oodac_typecheck_rejects_nested_field_call_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/nested_field_call_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_nested_field_call_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/nested_field_call_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn oodac_typecheck_rejects_return_field_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/return_field_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_return_field_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/return_field_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn oodac_typecheck_rejects_assign_field_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/assign_field_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_rejects_struct_lit_field_type_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/struct_lit_field_type_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_struct_lit_field_type_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/struct_lit_field_type_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn oodac_typecheck_rejects_nested_field_binop_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/nested_field_binop_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_nested_field_binop_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/nested_field_binop_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn oodac_typecheck_rejects_nested_field_missing() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/nested_field_missing.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_rejects_missing_struct_field() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/missing_struct_field.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn stage0_typecheck_rejects_missing_struct_field() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/missing_struct_field.oo");
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_rejects_nested_field_if_int() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/nested_field_if_int.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

