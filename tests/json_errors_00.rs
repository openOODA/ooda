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

