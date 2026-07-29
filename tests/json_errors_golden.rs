//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

#[test]
fn json_errors_golden_capability_violation() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/unauthorized_io.oo");
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
    assert!(
        !v["suggested_fix"]["diff"]
            .as_str()
            .unwrap_or("")
            .is_empty(),
        "suggested_fix.diff must be non-empty"
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
fn ooda_em_is_measured_not_theater() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/hello.oo");
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
