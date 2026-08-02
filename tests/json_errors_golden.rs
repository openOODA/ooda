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
    let diff = v["suggested_fix"]["diff"].as_str().unwrap_or("");
    assert!(
        diff.contains("missing_return") && diff.contains("declared_return"),
        "missing_return codemod must name declared return type: {}",
        diff
    );
    assert!(
        diff.contains("Int") && diff.contains("return 0"),
        "Int missing return stub: {}",
        diff
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

/// Content equality on WASM uses host `streq` (not pointer `i32.eq` alone).
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
#[test]
fn oodac_typecheck_call_lt_int_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/call_lt_int_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "call lt: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: `-g()` when g returns Bool fail-closed.
#[test]
fn oodac_typecheck_rejects_unary_minus_call_bool() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/unary_minus_call_bool.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `g() == true` when g returns Int fail-closed.
#[test]
fn oodac_typecheck_rejects_call_eq_lit_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_eq_lit_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `g() == 1` when g returns Int OK.
#[test]
fn oodac_typecheck_call_eq_lit_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/call_eq_lit_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `let x = g(); let y: String = x` when g returns Int.
#[test]
fn oodac_typecheck_rejects_let_call_bind_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/let_call_bind_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `let x = g(); let y: Int = x` OK.
#[test]
fn oodac_typecheck_let_call_bind_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/let_call_bind_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `!g()` when g returns Int fail-closed.
#[test]
fn oodac_typecheck_rejects_unary_bang_call() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/unary_bang_call.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `!g()` when g returns Bool OK.
#[test]
fn oodac_typecheck_unary_bang_call_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/unary_bang_call_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `if g()` when g returns Int fail-closed.
#[test]
fn oodac_typecheck_rejects_if_call_int_cond() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/if_call_int_cond.oo");
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

/// R1: `while g()` Int return fail-closed.
#[test]
fn oodac_typecheck_rejects_while_call_int_cond() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/while_call_int_cond.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `if g()` Bool return OK.
#[test]
fn oodac_typecheck_if_call_bool_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/if_call_bool_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `let x: Int = g()` when g returns String.
#[test]
fn oodac_typecheck_rejects_let_call_ann_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/let_call_ann_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `let x: Int = g()` OK when g returns Int.
#[test]
fn oodac_typecheck_let_call_ann_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/let_call_ann_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: lit-env `a && b` with Int and Bool fail-closed.
#[test]
fn oodac_typecheck_rejects_logic_var_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/logic_var_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: lit-env String < String order fail-closed.
#[test]
fn oodac_typecheck_rejects_cmp_var_string_lt() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/cmp_var_string_lt.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `return g()` type vs declared return.
#[test]
fn oodac_typecheck_rejects_return_call_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/return_call_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: matching `return g()` OK.
#[test]
fn oodac_typecheck_return_call_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/return_call_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `x = g()` type mismatch fail-closed.
#[test]
fn oodac_typecheck_rejects_assign_call_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/assign_call_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: matching `x = g()` OK.
#[test]
fn oodac_typecheck_assign_call_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/assign_call_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: `if x` with x pure-lit Int fail-closed.
#[test]
fn oodac_typecheck_rejects_if_var_int_cond() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/if_var_int_cond.oo");
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

/// R1: `if x` with x pure-lit Bool OK.
#[test]
fn oodac_typecheck_if_var_bool_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/if_var_bool_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// R1: unary `!x` with x pure-lit Int fail-closed.
#[test]
fn oodac_typecheck_rejects_unary_bang_var() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/unary_bang_var.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: unary `-x` with x pure-lit Bool fail-closed.
#[test]
fn oodac_typecheck_rejects_unary_minus_var() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/unary_minus_var.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: lit-env binop `a + b` with Int+String fail-closed.
#[test]
fn oodac_typecheck_rejects_binop_var_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/binop_var_mismatch.oo");
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
    assert!(c.contains("incompatible") || c.contains("ERR"), "{}", c);
}

/// R1: lit-env Int+Int binop OK.
#[test]
fn oodac_typecheck_binop_var_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/binop_var_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "binop var ok: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: lit-env `a == b` Int vs Bool fail-closed.
#[test]
fn oodac_typecheck_rejects_cmp_var_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/cmp_var_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: mut assign from typed var mismatch.
#[test]
fn oodac_typecheck_rejects_mut_assign_var() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/mut_assign_var.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: mut assign from matching typed var OK.
#[test]
fn oodac_typecheck_mut_assign_var_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/mut_assign_var_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
}

/// R1: `let x = 1; f(x)` when f expects String.
#[test]
fn oodac_typecheck_rejects_call_arg_var_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/call_arg_var_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `let x = 1; f(x)` OK when f expects Int.
#[test]
fn oodac_typecheck_call_arg_var_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/call_arg_var_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "call arg var: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: `return x` with x pure-lit Int vs String ret.
#[test]
fn oodac_typecheck_rejects_return_var_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/return_var_mismatch.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `return x` OK when types match via lit env.
#[test]
fn oodac_typecheck_return_var_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/return_var_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "return var: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: `let x = 1; let y: String = x` fail-closed (lit env).
#[test]
fn oodac_typecheck_rejects_var_init_ann_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/var_init_ann_mismatch.oo");
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
    assert!(c.contains("String") || c.contains("annotated") || c.contains("ERR"), "{}", c);
}

/// R1: `let x = 1; let y: Int = x` OK via lit env.
#[test]
fn oodac_typecheck_var_init_ann_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/var_init_ann_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "var init: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: param type alias accepts matching lit.
#[test]
fn oodac_typecheck_param_type_alias_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/param_type_alias.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "param alias: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: param type alias rejects wrong lit.
#[test]
fn oodac_typecheck_rejects_param_type_alias_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/param_type_alias_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: mut assign through type alias.
#[test]
fn oodac_typecheck_mut_type_alias_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/mut_type_alias.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "mut alias: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: mut assign wrong type through alias fail-closed.
#[test]
fn oodac_typecheck_rejects_mut_type_alias_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/mut_type_alias_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: return type alias `-> T` with `type T = Int` accepts `return 1`.
#[test]
fn oodac_typecheck_type_alias_return_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/type_alias_return.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "ret alias: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// R1: `-> T` still rejects wrong lit return.
#[test]
fn oodac_typecheck_rejects_type_alias_return_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/type_alias_return_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

/// R1: `<<` fail-closed.
#[test]
fn oodac_typecheck_rejects_shl_binop() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/shl_binop.oo");
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
    assert!(c.contains("unsupported") || c.contains("<<") || c.contains("ERR"), "{}", c);
}

/// R1: `>>` fail-closed.
#[test]
fn oodac_typecheck_rejects_shr_binop() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/shr_binop.oo");
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
    assert!(c.contains("unsupported") || c.contains(">>") || c.contains("ERR"), "{}", c);
}

/// R1: binary `&` fail-closed (not silent OK).
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
#[test]
fn oodac_typecheck_slice_rejects_div_by_zero() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/div_by_zero.oo");
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
    assert!(!out.status.success(), "oodac must reject /0");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.to_lowercase().contains("zero") || combined.to_lowercase().contains("type"),
        "got: {}",
        combined
    );
}

/// R1 expand: oodac must fail-closed on undefined variables (was silent OK).
#[test]
fn oodac_typecheck_slice_rejects_undefined_var() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/undefined_var.oo");
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
    assert!(
        !out.status.success(),
        "oodac must reject undefined var: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("undefined") || combined.to_lowercase().contains("type"),
        "expected undefined type error, got: {}",
        combined
    );
}

/// R1 expand: oodac must fail-closed on pure lit Int+String (was silent OK).
#[test]
fn oodac_typecheck_slice_rejects_int_string_binop() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/binop_int_string.oo");
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
    assert!(
        !out.status.success(),
        "oodac must reject Int+String: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("ERR") && combined.to_lowercase().contains("type"),
        "expected ERR type, got: {}",
        combined
    );
}

/// R1: oodac check (real .oo) must fail-closed on annotated-let type mismatch
/// (stage-0 previously rejected; oodac used to print OK — honesty bug).
#[test]
fn oodac_typecheck_slice_rejects_let_ann_mismatch() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/let_ann_mismatch.oo");
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
    assert!(
        !out.status.success(),
        "oodac must fail-closed on type mismatch: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("ERR") && combined.to_lowercase().contains("type"),
        "expected ERR type, got: {}",
        combined
    );
}

/// String literals → data segment + println_str; String `+` is bump-heap concat;
/// non-Add string arithmetic still fails closed (no silent pointer math).
#[test]
fn build_wasm_string_literal_println_and_refuses_concat() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wstr_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let ok_path = dir.join("ok.oo");
    std::fs::write(
        &ok_path,
        r#"
pub fn main() {
    println("hi");
    let s = "hi";
    println(s);
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", ok_path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm string println: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(ok_path.with_extension("wat")).expect("wat");
    assert!(wat.contains("println_str"), "wat:\n{}", wat);
    // Interned: only one "hi" data segment
    assert_eq!(
        wat.matches("(data (i32.const").count(),
        1,
        "expected interned single data segment:\n{}",
        wat
    );

    // String + is real concat (bump heap).
    let concat = dir.join("concat.oo");
    std::fs::write(
        &concat,
        r#"
pub fn main() {
    let a = "a";
    let b = "b";
    let c = a + b;
    println(c);
}
"#,
    )
    .unwrap();
    let out_cat = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", concat.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out_cat.status.success(),
        "string concat must lower on WASM: {}",
        String::from_utf8_lossy(&out_cat.stderr)
    );
    let cat_wat = std::fs::read_to_string(concat.with_extension("wat")).expect("wat");
    assert!(
        cat_wat.contains("global.get $heap"),
        "concat needs heap:\n{}",
        cat_wat
    );

    // List[String] now lowers; sealed caps still refuse on WASM.
    let ok_list = dir.join("lstr.oo");
    std::fs::write(
        &ok_list,
        r#"
pub fn main() {
    let mut xs: List[String] = list_new();
    xs = xs.push("a");
    println(xs.len());
}
"#,
    )
    .unwrap();
    let out_ok = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", ok_list.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out_ok.status.success(),
        "List[String] must lower on WASM: {}",
        String::from_utf8_lossy(&out_ok.stderr)
    );

    let bad = dir.join("bad.oo");
    std::fs::write(
        &bad,
        r#"
pub fn main(fs: &FsCap) {
    let s = fs.read_file("x");
    println(s);
}
"#,
    )
    .unwrap();
    let out2 = std::process::Command::new(bin)
        .args(["build", "--target", "wasm", bad.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        !out2.status.success(),
        "sealed FsCap must fail-closed on WASM"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out2.stderr),
        String::from_utf8_lossy(&out2.stdout)
    );
    assert!(
        err.contains("sealed")
            || err.contains("read_file")
            || err.contains("capability")
            || err.contains("FsCap"),
        "err={}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `for i in lo..hi` desugars to while in the parser; WASM must lower that path
/// (unique $break_N/$continue_N labels) without claiming a full WASM product.
#[test]
fn build_wasm_range_for_lowers_via_while() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_wfor_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("for.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut s = 0;
    for i in 0..3 {
        s = s + i;
    }
    println(s);
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
        "wasm range-for build: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).expect("wat");
    assert!(
        wat.contains("(loop") || wat.contains("loop $"),
        "expected while/loop in WAT from for desugar:\n{}",
        wat
    );
    assert!(
        wat.contains("$break_") || wat.contains("br "),
        "expected break label machinery in nested-capable loops:\n{}",
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
fn build_c_refuses_sys_exec_method() {
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
        !out.status.success(),
        "C must refuse sealed .sys_exec without runtime cap tokens"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains(".sys_exec") || err.contains("sys_exec"),
        "expected sealed refuse, got: {}",
        err
    );
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
fn build_c_refuses_path_exists_method() {
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
        !out.status.success(),
        "C must refuse sealed .path_exists without runtime cap tokens"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("path_exists"),
        "expected sealed refuse, got: {}",
        err
    );
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
fn build_c_refuses_file_size_method() {
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
        !out.status.success(),
        "C must refuse sealed .file_size without runtime cap tokens"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("file_size"),
        "expected sealed refuse, got: {}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
}

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
fn build_c_refuses_env_get_method() {
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
        !out.status.success(),
        "C must refuse sealed .env_get without runtime cap tokens"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("env_get"),
        "expected sealed refuse, got: {}",
        err
    );
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

#[test]
fn for_range_loop_run_and_c() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_for_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut s = 0;
    for i in 0..4 {
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
    assert!(run.status.success(), "run: {}", String::from_utf8_lossy(&run.stderr));
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("6"), "0+1+2+3=6 stdout={}", stdout);

    let build = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "C for desugar: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let exe = path.with_extension("");
    let out = std::process::Command::new(&exe).output().expect("exe");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("6"), "C stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}



#[test]
fn pkg_install_refuses_unsupported_remote() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    for url in [
        "https://example.com/pkg.zip",
        "http://example.com/not_tarball.rar",
    ] {
        let out = std::process::Command::new(bin)
            .args(["pkg", "--install", url])
            .output()
            .expect("spawn");
        assert!(!out.status.success(), "must fail: {}", url);
        let err = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(
            err.contains("tar.gz") || err.contains("tgz"),
            "url={} err={}",
            url,
            err
        );
    }
}


#[test]
fn where_non_const_fails_check() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_where_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(&path, "type Port = Int where x..y;\npub fn main() {}\n").unwrap();
    let out = std::process::Command::new(bin)
        .args(["check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(err.contains("where") || err.contains("const"), "{}", err);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn for_list_iteration_run_and_c() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_listfor_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut xs = list_new();
    xs = list_push(xs, 10);
    xs = list_push(xs, 20);
    let mut s = 0;
    for x in xs {
        s = s + x;
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
    assert!(
        run.status.success(),
        "list for run: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("30"), "stdout={}", stdout);

    let build = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "list for C: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let exe = path.with_extension("");
    let out = std::process::Command::new(&exe).output().expect("exe");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("30"), "C stdout={}", stdout);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn for_string_list_run_and_c() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_sfor_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut xs = list_new();
    xs = list_push(xs, "a");
    xs = list_push(xs, "b");
    let mut out = "";
    for x in xs {
        out = out + x;
    }
    println(out);
}
"#,
    )
    .unwrap();
    let run = std::process::Command::new(bin)
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("run");
    assert!(
        run.status.success(),
        "string list for run: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("ab"),
        "stdout={}",
        String::from_utf8_lossy(&run.stdout)
    );

    let build = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "string list for C: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let exe = path.with_extension("");
    let out = std::process::Command::new(&exe).output().expect("exe");
    assert!(out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("ab"),
        "C stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

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

#[test]
fn oodac_typecheck_rejects_nested_field_logic_bad() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/fail/nested_field_logic_bad.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
}

#[test]
fn oodac_typecheck_nested_field_cmp_logic_ok() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap/corpus/typecheck/pass/nested_field_cmp_logic_ok.oo");
    let oodac = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("oodac/main.oo");
    let out = std::process::Command::new(bin)
        .args(["run", oodac.to_str().unwrap(), "--", "check", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
}

/// CHS frontend parity: stage-0 vs oodac tokens/check on corpus (scripts/chs_parity.sh).
#[test]
fn chs_parity_script_passes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/chs_parity.sh");
    assert!(script.is_file(), "missing {}", script.display());
    let ooda = env!("CARGO_BIN_EXE_ooda");
    let out = std::process::Command::new("bash")
        .arg(script.to_str().unwrap())
        .env("OODA", ooda)
        .current_dir(root)
        .output()
        .expect("spawn chs_parity");
    let c = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "chs_parity failed:\n{}",
        c
    );
    assert!(c.contains("PASSED") || c.contains("OK"), "output:\n{}", c);
}
