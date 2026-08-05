//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn oodac_typecheck_rejects_amp_binop() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/amp_binop.oo");
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
    assert!(c.contains("unsupported") || c.contains("ERR"), "{}", c);
}

/// R1: binary `|` fail-closed.
#[test]
fn oodac_typecheck_rejects_pipe_binop() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/pipe_binop.oo");
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
    assert!(c.contains("unsupported") || c.contains("ERR"), "{}", c);
}

/// R1: `for i in 0..3` must not false-undefined `for`/`in`.
#[test]
fn oodac_typecheck_for_range_names_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/for_range_names.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "for range: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: `type T = Int; let x: T = 1` must OK (alias resolve).
#[test]
fn oodac_typecheck_type_alias_ann_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/type_alias_ann.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "type alias: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: `type T = Int; let x: T = true` still fail-closed.
#[test]
fn oodac_typecheck_rejects_type_alias_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/type_alias_mismatch.oo");
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
    assert!(c.contains("annotated") || c.contains("Bool") || c.contains("ERR"), "{}", c);
}

/// R1: `1 && true` fail-closed (Bool operands only).
#[test]
fn oodac_typecheck_rejects_logic_and_int() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/logic_and_int.oo");
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
    assert!(c.contains("Bool") || c.contains("logical") || c.contains("ERR"), "{}", c);
}

/// R1: `"a" || false` fail-closed.
#[test]
fn oodac_typecheck_rejects_logic_or_string() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/logic_or_string.oo");
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
    assert!(c.contains("Bool") || c.contains("logical") || c.contains("ERR"), "{}", c);
}

/// R1: true && false / true || false remain OK.
#[test]
fn oodac_typecheck_logic_bool_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/logic_bool_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "logic bool: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: unary `-` on String lit fail-closed.
#[test]
fn oodac_typecheck_rejects_unary_minus_string() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/unary_minus_string.oo");
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
    assert!(c.contains("unary") || c.contains("number") || c.contains("ERR"), "{}", c);
}

/// R1: String < String ordering compare fail-closed (numeric only).
#[test]
fn oodac_typecheck_rejects_cmp_string_lt() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/cmp_string_lt.oo");
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
    assert!(c.contains("numeric") || c.contains("comparison") || c.contains("ERR"), "{}", c);
}

/// R1: `if 1` pure-lit condition fail-closed.
#[test]
fn oodac_typecheck_rejects_if_int_cond() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/if_int_cond.oo");
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
    assert!(c.contains("Bool") || c.contains("condition") || c.contains("ERR"), "{}", c);
}

/// R1: while true { break } must not false-undefined break.
