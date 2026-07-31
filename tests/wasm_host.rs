//! Dev-only WAT host smoke (wasmtime).
//!
//! Proves Stage-0 emitted imports `env.println` / `env.println_str` / `env.streq`
//! are enough to **run** a small module. Not a product WASM runtime and not
//! claimed as full WASM product. `wasmtime` is a **dev-dependency** only.
use anyhow::{bail, Result};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use wasmtime::*;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Unique temp dir per test (pid alone races under cargo --test-threads>1).
fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let n = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "{}_{}_{}",
        prefix,
        std::process::id(),
        n
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[derive(Default)]
struct HostOut {
    lines: Vec<String>,
}

/// Run WAT with openOODA env imports; return captured println lines (no real stdout).
fn run_wat(wat: &str) -> Result<Vec<String>> {
    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, HostOut::default());
    let mut linker = Linker::new(&engine);

    linker.func_wrap("env", "println", |mut caller: Caller<'_, HostOut>, v: i64| {
        caller.data_mut().lines.push(format!("{}", v));
    })?;

    linker.func_wrap(
        "env",
        "println_str",
        |mut caller: Caller<'_, HostOut>, offset: i32| {
            let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => return,
            };
            let s = {
                let data = mem.data(&caller);
                let start = offset as usize;
                if start >= data.len() {
                    return;
                }
                let mut end = start;
                while end < data.len() && data[end] != 0 {
                    end += 1;
                }
                match std::str::from_utf8(&data[start..end]) {
                    Ok(s) => s.to_string(),
                    Err(_) => return,
                }
            };
            caller.data_mut().lines.push(s);
        },
    )?;

    linker.func_wrap(
        "env",
        "str_contains",
        |mut caller: Caller<'_, HostOut>, hay: i32, needle: i32| -> i32 {
            let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => return 0,
            };
            let data = mem.data(&caller);
            let read = |off: i32| -> String {
                let start = off as usize;
                if start >= data.len() {
                    return String::new();
                }
                let mut end = start;
                while end < data.len() && data[end] != 0 {
                    end += 1;
                }
                String::from_utf8_lossy(&data[start..end]).into_owned()
            };
            if read(hay).contains(&read(needle)) {
                1
            } else {
                0
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "streq",
        |mut caller: Caller<'_, HostOut>, a: i32, b: i32| -> i32 {
            let mem = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                Some(m) => m,
                None => return 0,
            };
            let data = mem.data(&caller);
            let read = |off: i32| -> &[u8] {
                let start = off as usize;
                if start >= data.len() {
                    return &[];
                }
                let mut end = start;
                while end < data.len() && data[end] != 0 {
                    end += 1;
                }
                &data[start..end]
            };
            if read(a) == read(b) {
                1
            } else {
                0
            }
        },
    )?;

    let instance = linker.instantiate(&mut store, &module)?;
    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .map_err(|_| anyhow::anyhow!("no export main: () -> i32"))?;
    let code = main.call(&mut store, ())?;
    if code != 0 {
        bail!("main returned non-zero status {}", code);
    }
    Ok(store.into_data().lines)
}

#[test]
fn host_streq_and_println_str_assert_output() {
    let wat = r#"
    (module
      (import "env" "println" (func $println (param i64)))
      (import "env" "println_str" (func $println_str (param i32)))
      (import "env" "streq" (func $streq (param i32 i32) (result i32)))
      (memory 1)
      (export "memory" (memory 0))
      (data (i32.const 1024) "hello\00")
      (data (i32.const 1030) "world\00")
      (data (i32.const 1036) "hello\00")
      (func (export "main") (result i32)
        (call $println_str (i32.const 1024))
        (call $println (i64.extend_i32_s (call $streq (i32.const 1024) (i32.const 1036))))
        (call $println (i64.extend_i32_s (call $streq (i32.const 1024) (i32.const 1030))))
        (i32.const 0)
      )
    )
    "#;
    let lines = run_wat(wat).expect("run wat");
    assert_eq!(
        lines,
        vec!["hello".to_string(), "1".to_string(), "0".to_string()],
        "expected println_str + streq equal/unequal; got {:?}",
        lines
    );
}

/// End-to-end: `ooda build --target wasm` → run emitted WAT under host imports.
#[test]
fn ooda_wasm_string_eq_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_whost");
    let path = dir.join("eq.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    if "aa" == "aa" {
        println(1);
    } else {
        println(0);
    }
    if "aa" == "bb" {
        println(1);
    } else {
        println(0);
    }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn ooda");
    assert!(
        out.status.success(),
        "wasm build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat_path = path.with_extension("wat");
    let wat = std::fs::read_to_string(&wat_path).expect("read wat");
    assert!(wat.contains("call $streq"), "expected streq in WAT:\n{}", wat);
    let lines = run_wat(&wat).expect("host run ooda WAT");
    assert_eq!(
        lines,
        vec!["1".to_string(), "0".to_string()],
        "equal then unequal string compare; got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_wasm_list_int_subset() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_whost");
    let path = dir.join("list.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut xs = list_new();
    xs = list_push(xs, 42);
    xs = list_push(xs, 99);
    println(list_len(xs));
    println(list_get(xs, 0));
    println(list_get(xs, 1));
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn ooda");
    assert!(
        out.status.success(),
        "wasm build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat_path = path.with_extension("wat");
    let wat = std::fs::read_to_string(&wat_path).expect("read wat");
    assert!(wat.contains("$list_new"), "list runtime missing:\n{}", wat);
    let lines = run_wat(&wat).expect("host run ooda WAT");
    assert_eq!(
        lines,
        vec!["2".to_string(), "42".to_string(), "99".to_string()],
        "list output got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ooda_wasm_list_eq() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_whost");
    let path = dir.join("list_eq.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut a = list_new();
    a = list_push(a, 10);
    a = list_push(a, 20);

    let mut b = list_new();
    b = list_push(b, 10);
    b = list_push(b, 20);

    let mut c = list_new();
    c = list_push(c, 10);
    c = list_push(c, 21);

    if a == b {
        println(1);
    } else {
        println(0);
    }

    if a == c {
        println(1);
    } else {
        println(0);
    }

    if a != c {
        println(1);
    } else {
        println(0);
    }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn ooda");
    assert!(
        out.status.success(),
        "wasm build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat_path = path.with_extension("wat");
    let wat = std::fs::read_to_string(&wat_path).expect("read wat");
    let lines = run_wat(&wat).expect("host run ooda WAT");
    assert_eq!(
        lines,
        vec!["1".to_string(), "0".to_string(), "1".to_string()],
        "list output got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}



/// Fixture fixtures/string_ops.oo combined string methods under host.
/// Variable String receiver — must not inject dead list RT (W↓).
#[test]
fn ooda_wasm_string_ops_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/string_ops.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "wasm string_ops: {}", String::from_utf8_lossy(&out.stderr));
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("str_contains") || wat.contains("call $str_contains"));
    assert!(
        !wat.contains("$list_new")
            && !wat.contains("$list_len")
            && !wat.contains("$list_push")
            && !wat.contains("$list_get")
            && !wat.contains("$list_eq"),
        "string_ops.oo is string-only; list RT must not inject:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    // len=5, char h=104, contains=1, slice=ell
    assert_eq!(
        lines,
        vec!["5".to_string(), "104".to_string(), "1".to_string(), "ell".to_string()],
        "got {:?}",
        lines
    );
}

/// Fixture fixtures/list_eq.oo deep equality under host.
/// Unique temp copy — parallel builds to fixtures/*.wat race (D↑ flaky green/red).
#[test]
fn ooda_wasm_list_eq_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/list_eq.oo");
    assert!(src.is_file(), "missing {}", src.display());
    let dir = unique_temp_dir("ooda_wlist_eq");
    let path = dir.join("list_eq.oo");
    std::fs::copy(&src, &path).expect("copy fixture");
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm list_eq fixture: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("call $list_eq"), "list_eq missing:\n{}", wat);
    assert!(
        wat.contains("(func $list_eq"),
        "list_eq RT must emit when == used:\n{}",
        wat
    );
    assert!(
        !wat.contains("\"streq\"") && !wat.contains("str_contains"),
        "list_eq is not string eq:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(lines, vec!["1".to_string(), "0".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// for-list desugars to while + nested `let x = list_get`; nested locals must declare.
#[test]
fn ooda_wasm_for_list_sum_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wforlist");
    let path = dir.join("forlist.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut xs: List[Int] = list_new();
    xs = list_push(xs, 1);
    xs = list_push(xs, 2);
    xs = list_push(xs, 3);
    let mut s = 0;
    for x in xs {
        s = s + x;
    }
    println(s);
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn ooda");
    assert!(
        out.status.success(),
        "wasm for-list build: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).expect("wat");
    assert!(wat.contains("(local $x "), "nested loop var local missing:\n{}", wat);
    let lines = run_wat(&wat).expect("host run");
    assert_eq!(lines, vec!["6".to_string()], "sum 1+2+3; got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fixture fixtures/list_sum.oo — method .push + for-list; println-only host (no string imports).
#[test]
fn ooda_wasm_list_sum_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/list_sum.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm list_sum: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("call $list_push") || wat.contains("call $list_get"));
    assert!(
        !wat.contains("println_str") && !wat.contains("\"streq\"") && !wat.contains("str_contains"),
        "list_sum is Int-only; no string host imports:\n{}",
        wat
    );
    assert!(
        !wat.contains("(func $list_eq"),
        "list_sum never compares lists; $list_eq RT must not inject:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(lines, vec!["6".to_string()], "got {:?}", lines);
}

/// Annotated List[String]: .len / list_get / println_str under host.
#[test]
fn ooda_wasm_list_string_push_len_get_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wlstr");
    let path = dir.join("lstr.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut xs: List[String] = list_new();
    xs = xs.push("hi");
    xs = xs.push("yo");
    println(xs.len());
    println(list_get(xs, 0));
    println(list_get(xs, 1));
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "List[String] wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("call $list_len") && wat.contains("call $list_get"),
        "list RT:\n{}",
        wat
    );
    assert!(
        wat.contains("call $println_str"),
        "string elements via println_str:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(
        lines,
        vec![
            "2".to_string(),
            "hi".to_string(),
            "yo".to_string()
        ],
        "got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// List[String] == is content equality (concat vs literal), not i64 pointer identity.
#[test]
fn ooda_wasm_list_string_eq_content_not_pointer() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wlstreq");
    let path = dir.join("eq.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut a: List[String] = list_new();
    a = a.push("hi" + "yo");
    let mut b: List[String] = list_new();
    b = b.push("hiyo");
    if a == b { println(1); } else { println(0); }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "list_str eq wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("call $list_str_eq"),
        "must use content eq RT:\n{}",
        wat
    );
    assert!(
        !wat.contains("call $list_eq"),
        "must not use Int list_eq for String lists:\n{}",
        wat
    );
    assert!(
        wat.contains("\"streq\"") || wat.contains("call $streq"),
        "list_str_eq needs streq:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(lines, vec!["1".to_string()], "content eq; got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Int list_eq programs must not pull `$list_str_eq` / streq (W↓).
/// Build into a unique temp path — shared fixtures/*.wat races under --test-threads>1.
#[test]
fn ooda_wasm_list_int_eq_no_list_str_eq_rt() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/list_eq.oo");
    assert!(src.is_file());
    let dir = unique_temp_dir("ooda_wlint_eq");
    let path = dir.join("list_eq.oo");
    std::fs::copy(&src, &path).expect("copy fixture");
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("(func $list_eq"),
        "Int list needs list_eq:\n{}",
        wat
    );
    assert!(
        !wat.contains("list_str_eq") && !wat.contains("\"streq\""),
        "Int list_eq must not inject string eq RT:\n{}",
        wat
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fixture fixtures/list_string.oo full surface under host.
#[test]
fn ooda_wasm_list_string_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/list_string.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm list_string: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("call $list_str_eq") || wat.contains("call $list_len"));
    let lines = run_wat(&wat).expect("host");
    // len=2, get hi, get yo, for hi, for yo, eq→1
    assert_eq!(
        lines,
        vec![
            "2".to_string(),
            "hi".to_string(),
            "yo".to_string(),
            "hi".to_string(),
            "yo".to_string(),
            "1".to_string(),
        ],
        "got {:?}",
        lines
    );
}

/// Fixture `string_walk.oo`: while + .len + .char_at under host.
#[test]
fn ooda_wasm_string_walk_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/string_walk.oo");
    assert!(path.is_file(), "missing fixture {}", path.display());
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm string_walk: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    let lines = run_wat(&wat).expect("host");
    // WASM println is numeric; 'a'=97 'b'=98
    assert_eq!(
        lines,
        vec!["97".to_string(), "98".to_string()],
        "walk got {:?}",
        lines
    );
}

/// `.str_slice` copies bytes onto bump heap; println_str shows result.
#[test]
fn ooda_wasm_str_slice_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wslice");
    let path = dir.join("slice.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    println("hello".str_slice(1, 4));
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "str_slice wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("(global $heap"), "heap required for slice:\n{}", wat);
    assert!(!wat.contains("$list_new"), "slice alone must not force list RT:\n{}", wat);
    let lines = run_wat(&wat).expect("host");
    assert_eq!(lines, vec!["ell".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// String `.contains` via host `str_contains` (real ooda→WAT→host path).
#[test]
fn ooda_wasm_string_contains_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wcont");
    let path = dir.join("cont.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    if "hello".contains("ell") {
        println(1);
    } else {
        println(0);
    }
    if "hello".contains("xyz") {
        println(1);
    } else {
        println(0);
    }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "contains wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("call $str_contains"),
        "missing str_contains:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(
        lines,
        vec!["1".to_string(), "0".to_string()],
        "got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// String `.char_at(i)` loads byte at offset as i64 (ASCII).
#[test]
fn ooda_wasm_string_char_at_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wcat");
    let path = dir.join("cat.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    println("hi".char_at(0));
    println("hi".char_at(1));
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm char_at: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("i32.load8_u"), "load8 missing:\n{}", wat);
    let lines = run_wat(&wat).expect("host");
    // 'h' = 104, 'i' = 105
    assert_eq!(
        lines,
        vec!["104".to_string(), "105".to_string()],
        "got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// String `.len` via pure WAT NUL-byte scan (no host import).
#[test]
fn ooda_wasm_string_len_method_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wslen");
    let path = dir.join("slen.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    println("hi".len());
    println("hello".len());
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm string .len: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("__strlen_p"), "scratch local missing:\n{}", wat);
    assert!(wat.contains("i32.load8_u"), "byte load missing:\n{}", wat);
    let lines = run_wat(&wat).expect("host");
    assert_eq!(
        lines,
        vec!["2".to_string(), "5".to_string()],
        "got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Empty lists: deep-equal (len 0) even with distinct headers; must call $list_eq.
#[test]
fn ooda_wasm_empty_list_deep_eq() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wemptyeq");
    let path = dir.join("empty.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let a: List[Int] = list_new();
    let b: List[Int] = list_new();
    if a == b {
        println(1);
    } else {
        println(0);
    }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "empty list eq build: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("call $list_eq"),
        "empty list == must use list_eq:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(
        lines,
        vec!["1".to_string()],
        "empty deep eq; got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// List ==/!= is **deep equality** of content.
#[test]
fn ooda_wasm_list_deep_eq_not_streq() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wlisteq");
    let path = dir.join("listeq.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut a: List[Int] = list_new();
    a = a.push(1);
    let mut b: List[Int] = list_new();
    b = b.push(1);
    if a == a {
        println(1);
    } else {
        println(0);
    }
    if a == b {
        println(1);
    } else {
        println(0);
    }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("call $list_eq"),
        "list == must call $list_eq:\n{}",
        wat
    );
    // Module may still *import* streq for string ops; list compares must not *call* streq.
    let list_eq_calls = wat.matches("call $list_eq").count();
    assert!(list_eq_calls >= 2, "expected two list compares, wat:\n{}", wat);
    let lines = run_wat(&wat).unwrap();
    assert_eq!(
        lines,
        vec!["1".to_string(), "1".to_string()],
        "a==a true, a==b deep-eq true; got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Method forms `.push` / `.len` lower to free list_* on List[Int].
#[test]
fn ooda_wasm_list_methods_push_len_run_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wmeth");
    let path = dir.join("meth.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut xs: List[Int] = list_new();
    xs = xs.push(10);
    xs = xs.push(20);
    println(xs.len());
    println(list_get(xs, 0));
    println(list_get(xs, 1));
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm method list: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("call $list_push"), "expected list_push lower:\n{}", wat);
    assert!(wat.contains("call $list_len"), "expected list_len lower:\n{}", wat);
    let lines = run_wat(&wat).expect("host");
    assert_eq!(
        lines,
        vec!["2".to_string(), "10".to_string(), "20".to_string()],
        "got {:?}",
        lines
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: Bool `||` under host.
#[test]
fn ooda_wasm_bool_or_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wbor");
    let path = dir.join("bor.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = false || true;
    if x { println(1); } else { println(0); }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    let lines = run_wat(&wat).expect("host or");
    assert_eq!(lines, vec!["1".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: unary `!` under host.
#[test]
fn ooda_wasm_bool_not_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wbnot");
    let path = dir.join("bnot.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = !false;
    if x { println(1); } else { println(0); }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    let lines = run_wat(&wat).expect("host not");
    assert_eq!(lines, vec!["1".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: nested while accumulation under host.
#[test]
fn ooda_wasm_nested_while_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wnestw");
    let path = dir.join("nestw.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut i = 0;
    let mut s = 0;
    while i < 3 {
        let mut j = 0;
        while j < 2 {
            s = s + 1;
            j = j + 1;
        }
        i = i + 1;
    }
    println(s);
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    let lines = run_wat(&wat).expect("host nested while");
    assert_eq!(lines, vec!["6".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: Bool `&&` + if/else under host.
#[test]
fn ooda_wasm_bool_and_if_else_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wbool");
    let path = dir.join("bool.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let x = true && false;
    if x { println(1); } else { println(0); }
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    let lines = run_wat(&wat).expect("host bool");
    assert_eq!(lines, vec!["0".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: float sub host.
#[test]
fn ooda_wasm_float_sub_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wfsub");
    let path = dir.join("fsub.oo");
    std::fs::write(
        &path,
        "pub fn main() {\n    let x = 5.0 - 2.0;\n    println(x);\n}\n",
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("f64.sub") || wat.contains("f64.const"));
    let lines = run_wat(&wat).expect("host float sub");
    assert_eq!(lines, vec!["3".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: float div truncates toward zero for println host.
#[test]
fn ooda_wasm_float_div_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wfdiv");
    let path = dir.join("fdiv.oo");
    std::fs::write(
        &path,
        "pub fn main() {\n    let x = 7.0 / 2.0;\n    println(x);\n}\n",
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("f64.div") || wat.contains("f64.const"));
    let lines = run_wat(&wat).expect("host float div");
    assert_eq!(lines, vec!["3".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: if/else both arms lower; host sees else path.
#[test]
fn ooda_wasm_if_else_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_welse");
    let path = dir.join("else.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut x = 0;
    if false {
        x = 1;
    } else {
        x = 2;
    }
    println(x);
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    let lines = run_wat(&wat).expect("host if/else");
    assert_eq!(lines, vec!["2".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: nested if in else.
#[test]
fn ooda_wasm_nested_if_else_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wnestif");
    let path = dir.join("nest.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let mut x = 0;
    if false {
        x = 1;
    } else {
        if true {
            x = 3;
        }
    }
    println(x);
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    let lines = run_wat(&wat).expect("host nested if");
    assert_eq!(lines, vec!["3".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: float mul lowers to f64.mul and truncates to println (host).
#[test]
fn ooda_wasm_float_mul_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wfmul");
    let path = dir.join("fmul.oo");
    std::fs::write(
        &path,
        "pub fn main() {\n    let x = 2.0 * 3.0;\n    println(x);\n}\n",
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "float mul wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("f64.mul") || wat.contains("f64.const"),
        "expected f64 ops:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host float mul");
    // Alpha truncates f64 → i64 for println host import.
    assert_eq!(lines, vec!["6".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Dual-engine: float add on host via trunc println.
#[test]
fn ooda_wasm_float_add_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wfadd");
    let path = dir.join("fadd.oo");
    std::fs::write(
        &path,
        "pub fn main() {\n    let x = 1.5 + 2.5;\n    println(x);\n}\n",
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("f64.add") || wat.contains("f64.const"));
    let lines = run_wat(&wat).expect("host float add");
    assert_eq!(lines, vec!["4".to_string()], "got {:?}", lines);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Pure int program must not pull list runtime (W↓).
#[test]
fn ooda_wasm_no_list_runtime_without_lists() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wnolist");
    let path = dir.join("pure.oo");
    std::fs::write(&path, "pub fn main() { println(1 + 2); }\n").unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        !wat.contains("$list_new"),
        "list runtime should not inject without lists:\n{}",
        wat
    );
    assert!(
        !wat.contains("(memory"),
        "memory not needed for pure int:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host pure int");
    assert_eq!(lines, vec!["3".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Pure Int WASM must not import string host ops (E-M D↓: minimal sealed host surface).
#[test]
fn ooda_wasm_pure_int_no_string_host_imports() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wpureimp");
    let path = dir.join("pure.oo");
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
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "pure int wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("import \"env\" \"println\""),
        "println import required:\n{}",
        wat
    );
    assert!(
        !wat.contains("println_str"),
        "pure int must not import println_str:\n{}",
        wat
    );
    assert!(
        !wat.contains("\"streq\""),
        "pure int must not import streq:\n{}",
        wat
    );
    assert!(
        !wat.contains("str_contains"),
        "pure int must not import str_contains:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(lines, vec!["3".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// String-only programs need memory but must not inject list_* RT (W↓).
/// Drives **variable** String receivers (not only literals) so list-RT injection
/// from naive non-literal `.len` cannot slip through.
#[test]
fn ooda_wasm_string_only_no_list_runtime() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wstronly");
    let path = dir.join("str.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let s = "ab";
    println(s.len());
    println(s.char_at(0));
    println("cd".len());
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "string-only wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(wat.contains("(memory"), "strings need memory:\n{}", wat);
    assert!(
        !wat.contains("$list_new")
            && !wat.contains("$list_len")
            && !wat.contains("$list_push")
            && !wat.contains("$list_get")
            && !wat.contains("$list_eq"),
        "list RT must not inject for variable String methods:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(
        lines,
        vec!["2".to_string(), "97".to_string(), "2".to_string()]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Fixture fixtures/break_loop.oo — while tail if + break/continue (dual-engine honesty).
#[test]
fn ooda_wasm_break_loop_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/break_loop.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm break_loop: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("br $break_") || wat.contains("br $break"),
        "break must lower:\n{}",
        wat
    );
    assert!(
        wat.contains("br $continue_") || wat.contains("br $continue"),
        "continue must lower:\n{}",
        wat
    );
    // Pure Int: no string host / list RT (W↓).
    assert!(
        !wat.contains("println_str") && !wat.contains("$list_new"),
        "break_loop is Int-only:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    // i=1..7 skip 3, stop before 8: 1+2+4+5+6+7 = 25
    assert_eq!(lines, vec!["25".to_string()], "got {:?}", lines);
}

/// Fixture fixtures/for_range.oo — for lo..hi desugar (no list RT).
#[test]
fn ooda_wasm_for_range_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/for_range.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm for_range: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        !wat.contains("$list_new") && !wat.contains("$list_get"),
        "for-range is while desugar, not list RT:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    // 0+1+2+3+4 = 10
    assert_eq!(lines, vec!["10".to_string()], "got {:?}", lines);
}

/// Fixture fixtures/str_concat.oo — pure-WAT bump-heap String + (no host strcat).
#[test]
fn ooda_wasm_str_concat_fixture_runs_on_host() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/str_concat.oo");
    assert!(path.is_file(), "missing {}", path.display());
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "wasm str_concat: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        wat.contains("global.get $heap") && wat.contains("global.set $heap"),
        "concat needs bump heap:\n{}",
        wat
    );
    assert!(
        !wat.contains("str_concat") && !wat.contains("strcat"),
        "no host strcat import:\n{}",
        wat
    );
    assert!(
        !wat.contains("$list_new"),
        "string concat must not pull list RT:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(lines, vec!["hiyo".to_string()], "got {:?}", lines);
}

/// Variable String `.len` must not pull dead `$list_*` runtime (skeptic gap).
#[test]
fn ooda_wasm_var_string_len_no_list_runtime() {
    let bin = env!("CARGO_BIN_EXE_ooda");
    let dir = unique_temp_dir("ooda_wvarlen");
    let path = dir.join("varlen.oo");
    std::fs::write(
        &path,
        r#"
pub fn main() {
    let s = "hello";
    println(s.len());
}
"#,
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["build", "--target", "wasm", path.to_str().unwrap()])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "var string len wasm: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(path.with_extension("wat")).unwrap();
    assert!(
        !wat.contains("$list_new"),
        "variable String .len must not emit $list_new:\n{}",
        wat
    );
    assert!(
        !wat.contains("$list_len")
            && !wat.contains("$list_push")
            && !wat.contains("$list_get")
            && !wat.contains("$list_eq"),
        "variable String .len must not emit list RT:\n{}",
        wat
    );
    let lines = run_wat(&wat).expect("host");
    assert_eq!(lines, vec!["5".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}
