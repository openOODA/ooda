//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn oodac_typecheck_while_true_break_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/while_true_break.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "while/break: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: mut assign pure-lit RHS must match mut var type (stage-0 parity).
#[test]
fn oodac_typecheck_rejects_mut_assign_type() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/mut_assign_type.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn oodac");
    assert!(!out.status.success(), "mut assign type mismatch must fail");
    let c = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        c.contains("cannot assign") || c.contains("String") || c.contains("ERR"),
        "got {}",
        c
    );
}

/// R1: unary `!` on Int lit must fail-closed (stage-0 parity).
#[test]
fn oodac_typecheck_rejects_unary_bang_int() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/unary_bang_int.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn oodac");
    assert!(!out.status.success(), "unary !1 must fail");
    let c = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        c.contains("Bool") || c.contains("unary") || c.contains("ERR"),
        "got {}",
        c
    );
}

/// R1: well-typed mut reassign remains OK.
#[test]
fn oodac_typecheck_mut_assign_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/mut_assign_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn oodac");
    assert!(
        out.status.success(),
        "mut assign ok: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: match Ok(v)/Err(e) pattern binds must not false-undefined.
#[test]
fn oodac_typecheck_rejects_arg_type_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/arg_type_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output().unwrap();
    assert!(!out.status.success());
    let c=format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(c.contains("expects")||c.contains("ERR")||c.contains("String"), "{}", c);
}

#[test]
fn oodac_typecheck_rejects_must_use_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/must_use_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output().unwrap();
    assert!(!out.status.success());
    let c=format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(c.contains("must-use")||c.contains("unused")||c.contains("ERR"), "{}", c);
}

#[test]
fn oodac_typecheck_rejects_missing_return() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/missing_return.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output().expect("spawn");
    assert!(!out.status.success());
    let c = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(c.contains("missing return") || c.contains("ERR"), "{}", c);
}

#[test]
fn oodac_typecheck_rejects_nested_let_leak() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/nested_let_leak.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output().expect("spawn");
    assert!(!out.status.success(), "nested let leak must fail");
    let c = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(c.contains("undefined"), "got {}", c);
}

#[test]
fn oodac_typecheck_match_pattern_bind_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/match_bind_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args([
            "run",
            oodac.to_str().unwrap(),
            "--",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn oodac");
    assert!(
        out.status.success(),
        "match bind should OK: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: call arity + immut assign fail-closed via real oodac check.
#[test]
fn oodac_typecheck_slice_rejects_call_arity() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    for name in ["call_arity_few.oo", "call_arity_many.oo", "immut_assign.oo"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("bootstrap/corpus/typecheck/fail")
            .join(name);
        assert!(path.is_file(), "missing {}", path.display());
        let out = std::process::Command::new(bin)
            .args([
                "run",
                oodac.to_str().unwrap(),
                "--",
                "check",
                path.to_str().unwrap(),
            ])
            .output()
            .expect("spawn oodac");
        assert!(
            !out.status.success(),
            "{} must fail: {}",
            name,
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

/// R1: float literal /0.0 fail-closed.
#[test]
fn oodac_typecheck_slice_rejects_float_div_by_zero() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/float_div_by_zero.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args([
            "run",
            oodac.to_str().unwrap(),
            "--",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn oodac");
    assert!(!out.status.success(), "oodac must reject float /0");
    let c = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        c.to_lowercase().contains("zero") || c.contains("ERR"),
        "got: {}",
        c
    );
}

/// R1: const integer /0 fail-closed.
