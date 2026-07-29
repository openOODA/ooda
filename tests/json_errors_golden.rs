//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn json_errors_golden_capability_violation() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/unauthorized_io.oo");
    let out = std::process::Command::new(bin)
        .args(["check", example, "--json-errors"])
        .output()
        .expect("spawn ooda check");
    assert!(
        !out.status.success(),
        "unauthorized_io.oo must fail non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("--json-errors stderr is not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(
        v["error_type"].as_str(),
        Some("CapabilitySecurityViolation"),
        "diag: {}",
        stderr
    );
    assert!(
        v["line"].as_u64().unwrap_or(0) >= 1,
        "line must be set: {}",
        stderr
    );
    let msg = v["message"].as_str().unwrap_or("");
    assert!(!msg.is_empty(), "message must be non-empty");
    assert!(
        msg.contains("fetch") || msg.contains("NetCap") || msg.contains("Capability"),
        "message should mention capability/fetch: {}",
        msg
    );
    assert!(
        v["suggested_fix"].is_object(),
        "suggested_fix required for AI auto-fix: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(!diff.is_empty(), "suggested_fix.diff must be non-empty");
    assert!(
        diff.contains("rogue_fetch") || diff.contains("NetCap") || diff.contains("fetch"),
        "cap fix should name the function or cap/effect: {}",
        diff
    );
    // Cap fixes are machine-applicable ooda-patch JSON (not advisory theater).
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "cap suggested_fix must be applicability=patch: {}",
        stderr
    );
    assert!(
        diff.contains("target_function") || diff.contains('{'),
        "patch applicability should look like ooda patch JSON: {}",
        diff
    );
    // Honesty: no hardcoded E-M theater.
    assert!(
        v.get("em_savings").is_none(),
        "em_savings must not be injected as fake telemetry: {}",
        stderr
    );
    // Real measured clocks must be present on the error path.
    assert!(
        v["timings_us"].is_object(),
        "timings_us required (measured parse/check µs): {}",
        stderr
    );
    assert!(
        v["timings_us"]["parse_us"].as_u64().is_some(),
        "timings_us.parse_us missing: {}",
        stderr
    );
    assert!(
        v["timings_us"]["check_us"].as_u64().is_some(),
        "timings_us.check_us missing: {}",
        stderr
    );
}

#[test]
fn json_errors_undefined_function_names_symbol() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_undef_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("undef.oo");
    std::fs::write(
        &path,
        "pub fn main() {\n    let x = totally_missing_fn(1);\n    println(x);\n}\n",
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn ooda check");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect(&stderr);
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("totally_missing_fn"),
        "fix should name the symbol: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_wasm_refuses_requires_contracts() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wasm_req_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("req.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int
    requires a >= 0
{
    return a + b;
}
pub fn main() {
    println(add(1, 2));
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn build wasm");
    assert!(
        !out.status.success(),
        "WASM build must refuse requires/ensures"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        err.contains("contracts") || err.contains("requires"),
        "honest message required: {}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_refuses_requires_contracts() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_req_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("req.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int
    requires a >= 0
{
    return a + b;
}
pub fn main() {
    println(add(1, 2));
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn build");
    assert!(
        !out.status.success(),
        "C build must refuse requires/ensures: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        err.contains("contracts") || err.contains("requires"),
        "honest message required: {}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_em_is_measured_not_theater() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/hello.oo");
    let out = std::process::Command::new(bin)
        .args(["em", example])
        .output()
        .expect("spawn ooda em");
    assert!(out.status.success(), "hello.oo em should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("measured"), "stdout: {}", stdout);
    assert!(stdout.contains("µs") || stdout.contains("us"), "stdout: {}", stdout);
    assert!(!stdout.contains("82.4"), "no fake savings: {}", stdout);
    assert!(
        !stdout.contains("OPTIMAL MANEUVERABILITY"),
        "no marketing floor: {}",
        stdout
    );
}

#[test]
fn ooda_em_fails_nonzero_when_check_fails() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_em_fail_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("bad.oo");
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
        .args(["em", path.to_str().unwrap()])
        .output()
        .expect("spawn em");
    assert!(!out.status.success(), "em must fail non-zero when check fails");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("measured") || stdout.contains("check_failed"), "stdout: {}", stdout);
    assert!(!stdout.contains("82.4"), "no theater: {}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unfinished_cli_lsp_pkg_replay_exit_nonzero() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    for args in [
        vec!["lsp"],
        vec!["pkg", "--install", "nope"],
        vec!["replay", "x.oo", "t"],
    ] {
        let out = std::process::Command::new(bin)
            .args(&args)
            .output()
            .expect("spawn");
        assert!(
            !out.status.success(),
            "unfinished {:?} must exit non-zero",
            args
        );
        let err = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(
            err.contains("not implemented") || err.contains("refused"),
            "honest message for {:?}: {}",
            args,
            err
        );
    }
}

#[test]
fn json_errors_missing_return_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_noret_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("noret.oo");
    std::fs::write(
        &path,
        r#"
pub fn f() -> Int {
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
    assert!(!out.status.success(), "missing return must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert!(
        v["message"].as_str().unwrap_or("").contains("missing return")
            || v["message"].as_str().unwrap_or("").contains("Void"),
        "msg: {}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "missing return should be patch-applicable: {}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_unreachable_after_return_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_unreach_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("u.oo");
    std::fs::write(
        &path,
        r#"
pub fn f() -> Int {
    return 1;
    let y = 2;
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
    assert!(!out.status.success(), "unreachable after return must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert!(
        v["message"].as_str().unwrap_or("").contains("unreachable"),
        "msg: {}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "unreachable should be patch-applicable: {}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_patch_cli_changes_return_type() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_patch_cli_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("p.oo");
    std::fs::write(
        &path,
        r#"
pub fn add(a: Int, b: Int) -> Int {
    return a + b;
}
"#,
    )
    .unwrap();
    let diff = r#"{"target_function":"add","new_return_type":"Float","new_body":"return 1.0;"}"#;
    let out = std::process::Command::new(bin)
        .args(["patch", path.to_str().unwrap(), "--diff", diff])
        .output()
        .expect("spawn patch");
    assert!(
        out.status.success(),
        "patch CLI must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(
        after.contains("-> Float") || after.contains("Float"),
        "return type patched: {}",
        after
    );
    assert!(
        after.contains("1.0"),
        "body patched: {}",
        after
    );
    // Patched program must still typecheck.
    let chk = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("check patched");
    assert!(
        chk.status.success(),
        "patched file must typecheck: {}",
        String::from_utf8_lossy(&chk.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_migrate_json_reports_let_mut_fix() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_mig_json_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = 1;
    x = 2;
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["migrate", path.to_str().unwrap(), "--edition", "2026", "--json"])
        .output()
        .expect("spawn migrate --json");
    assert!(out.status.success(), "migrate --json should succeed: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stdout)
    });
    assert_eq!(v["let_mut_fixes"].as_u64(), Some(1), "stdout: {}", stdout);
    assert_eq!(v["changed"].as_bool(), Some(true));
    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("let mut x"), "file rewritten: {}", after);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_em_json_is_measured_report_not_theater() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/em_demo.oo");
    let out = std::process::Command::new(bin)
        .args(["em", example, "--json"])
        .output()
        .expect("spawn ooda em --json");
    assert!(out.status.success(), "em --json should succeed: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("em --json stdout is not JSON: {}\n{}", e, stdout)
    });
    assert!(v["source_bytes"].as_u64().unwrap_or(0) > 0, "W missing: {}", stdout);
    assert!(v["parse_us"].as_u64().is_some(), "parse_us: {}", stdout);
    assert!(v["typecheck_us"].as_u64().is_some(), "typecheck_us: {}", stdout);
    assert!(v["total_us"].as_u64().unwrap_or(0) >= 1, "total_us: {}", stdout);
    assert!(v.get("em_savings").is_none(), "no theater: {}", stdout);
    assert!(!stdout.contains("82.4"), "no 82.4 theater: {}", stdout);
    assert_eq!(v["check_failed"].as_bool(), Some(false), "em_demo should check clean: {}", stdout);
}

#[test]
fn json_errors_immutable_assign_has_let_mut_patch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_imut_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("imut.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = 1;
    x = 2;
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "immutable assign must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    assert!(
        v["message"].as_str().unwrap_or("").contains("immutable")
            || v["message"].as_str().unwrap_or("").contains("let mut"),
        "msg: {}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "let-mut fix should be patch-applicable: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("let_mut") || diff.contains("let mut") || diff.contains("migrate"),
        "diff should mention let mut codemod: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_wasm_refuses_sealed_io_effects() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/chs_fs_roundtrip.oo");
    let out = std::process::Command::new(bin)
        .args(["build", example, "--target", "wasm"])
        .output()
        .expect("spawn ooda build wasm");
    assert!(!out.status.success(), "sealed FS must not compile to wasm without runtime caps");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("capability") || err.contains("read_file"),
        "expected sealed refuse, got: {}",
        err
    );
}

#[test]
fn build_c_refuses_sealed_io_effects() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/chs_fs_roundtrip.oo");
    let out = std::process::Command::new(bin)
        .args(["build", example, "--target", "c"])
        .output()
        .expect("spawn ooda build");
    assert!(!out.status.success(), "sealed FS must not compile to C without runtime caps");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("capability") || err.contains("read_file"),
        "expected sealed I/O refuse message, got: {}",
        err
    );
}

#[test]
fn build_llvm_and_native_refuse_sealed_io_effects() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/chs_fs_roundtrip.oo");
    for target in ["llvm", "native"] {
        let out = std::process::Command::new(bin)
            .args(["build", example, "--target", target])
            .output()
            .expect("spawn ooda build");
        assert!(
            !out.status.success(),
            "sealed FS must not compile to {} without runtime caps",
            target
        );
        let err = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(
            err.contains("sealed") || err.contains("capability") || err.contains("read_file"),
            "expected sealed refuse for {}: {}",
            target,
            err
        );
    }
}

#[test]
fn json_errors_arity_mismatch_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_arity_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("arity.oo");
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
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "arity mismatch must fail closed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert_eq!(v["error_type"].as_str(), Some("TypeError"));
    let msg = v["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("expects 2") || msg.contains("found 1"),
        "msg: {}",
        msg
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "arity fix should be patch-applicable: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("arg_count") && diff.contains("add"),
        "diff should be arg_count codemod for add: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

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

#[test]
fn json_errors_str_concat_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_sc_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("s.oo");
    std::fs::write(&path, "pub fn main() {\n    let s = \"a\" + 1;\n    println(s);\n}\n")
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
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "{}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(diff.contains("str_concat") || diff.contains("to_string"), "{}", diff);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_outline_json_is_structured() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/hello.oo");
    let out = std::process::Command::new(bin)
        .args(["outline", example, "--json"])
        .output()
        .expect("spawn outline --json");
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stdout)
    });
    assert!(v["functions"].as_array().map(|a| !a.is_empty()).unwrap_or(false));
    let names: Vec<_> = v["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["name"].as_str())
        .collect();
    assert!(names.contains(&"greet"), "funcs: {:?}", names);
    assert!(!stdout.contains("Binary {"));
}

#[test]
fn wrong_kind_cap_handle_message_names_kinds() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wk_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("wk.oo");
    std::fs::write(
        &path,
        r#"
pub fn mix(net: &NetCap, fs: &FsCap) {
    let r = write_file(net, "/tmp/x", "y");
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
        "wrong-kind message: {}",
        stderr
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_errors_param_refinement_oob_is_patch_applicable() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_pref_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("p.oo");
    std::fs::write(
        &path,
        r#"
pub fn port(p: Int[1..65535]) -> Int {
    return p;
}
pub fn main() {
    println(port(0));
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap(), "--json-errors"])
        .output()
        .expect("spawn check");
    assert!(!out.status.success(), "const OOB param refinement must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|e| {
        panic!("not JSON: {}\n{}", e, stderr)
    });
    assert!(
        v["message"]
            .as_str()
            .unwrap_or("")
            .contains("RefinementTypeViolation"),
        "msg: {}",
        stderr
    );
    assert_eq!(
        v["suggested_fix"]["applicability"].as_str(),
        Some("patch"),
        "refinement fix must be patch-applicable: {}",
        stderr
    );
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("refinement_bounds"),
        "codemod name: {}",
        diff
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nested_let_scope_roundtrip_run() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_scope_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("s.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = 1;
    if true {
        let x = 99;
        println(x);
    }
    println(x);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("spawn run");
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Nested prints 99, then outer must still print 1 (not 99).
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| *l == "99" || *l == "1")
        .collect();
    assert!(
        lines.iter().any(|l| *l == "99") && lines.last() == Some(&"1"),
        "expected nested 99 then outer 1, got lines {:?}\nfull:\n{}",
        lines,
        stdout
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_wasm_while_polarity_in_wat() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_ww_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("w.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut i = 0;
    while i < 3 {
        i = i + 1;
    }
    println(i);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn build wasm");
    assert!(
        out.status.success(),
        "wasm build: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).expect("wat");
    assert!(
        wat.contains("i64.eqz"),
        "while polarity i64.eqz missing:\n{}",
        wat
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn build_c_lowers_char_at_method() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_cat_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    println("hi".char_at(0));
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
        "C must lower .char_at: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains('h'), "stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

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
        "C must lower .sys_exec: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = path.with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run");
    assert!(run.status.success());
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
        "C must lower .path_exists: {}",
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
