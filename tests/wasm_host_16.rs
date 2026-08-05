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
