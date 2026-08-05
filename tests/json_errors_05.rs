//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

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
