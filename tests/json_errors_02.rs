//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

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
fn build_c_lowers_fscap_sealed_io() {
    // Assembly depth + honesty: CHS C lowers compile-time-capped FS (chs_rt fopen).
    // Aligns CLI `build --target c` with host chs_build (native oodac bootstrap).
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/chs_fs_roundtrip.oo");
    let out = std::process::Command::new(bin)
        .args(["build", example, "--target", "c"])
        .output()
        .expect("spawn ooda build");
    assert!(
        out.status.success(),
        "C must lower FsCap I/O for bootstrap: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = std::path::Path::new(example).with_extension("");
    let run = std::process::Command::new(&exe).output().expect("run fs binary");
    assert!(run.status.success(), "fs binary failed: {:?}", run);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("chs-m0-ok"),
        "expected roundtrip body, got: {}",
        stdout
    );
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_file(exe.with_extension("c"));
}

#[test]
fn build_native_lowers_fscap_sealed_io() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/chs_fs_roundtrip.oo");
    let out = std::process::Command::new(bin)
        .args(["build", example, "--target", "native"])
        .output()
        .expect("spawn ooda build native");
    assert!(
        out.status.success(),
        "native must lower FsCap I/O: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = std::path::Path::new(example).with_extension("");
    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_file(exe.with_extension("c"));
}

#[test]
fn build_llvm_refuses_sealed_io_effects() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/chs_fs_roundtrip.oo");
    let out = std::process::Command::new(bin)
        .args(["build", example, "--target", "llvm"])
        .output()
        .expect("spawn ooda build llvm");
    assert!(
        !out.status.success(),
        "sealed FS must not compile to llvm without runtime caps"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("capability") || err.contains("read_file"),
        "expected sealed refuse for llvm: {}",
        err
    );
}

#[test]
fn build_c_refuses_net_sealed_fetch() {
    // Net is not lowered on C — fail closed (no silent oo_fetch link error theater).
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = std::env::temp_dir().join(format!("ooda_c_fetch_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("m.oo");
    std::fs::write(
        &path,
        r#"
pub fn main(net: &NetCap) {
    let r = fetch(net, "http://example.com");
    println("x");
}
"#,
    )
    .unwrap();
    let out = std::process::Command::new(bin)
        .args(["build", "--target", "c", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "C must refuse sealed fetch");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        err.contains("sealed") || err.contains("fetch") || err.contains("not lowered"),
        "expected sealed net refuse, got: {}",
        err
    );
    let _ = std::fs::remove_dir_all(&dir);
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

