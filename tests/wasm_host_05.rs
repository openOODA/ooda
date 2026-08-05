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
