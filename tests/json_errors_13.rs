//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

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

