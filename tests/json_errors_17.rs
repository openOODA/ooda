//! Golden: `ooda check --json-errors` emits a parseable AI diagnostic
//! with error_type, line, message, suggested_fix, and measured timings_us.
//! Must not inject fake em_savings telemetry.

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

